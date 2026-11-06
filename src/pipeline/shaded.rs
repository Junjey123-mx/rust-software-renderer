//! Laboratory 6: programmable fragment stage on the existing pipeline.
//!
//! `render_shaded` reuses the vertex shader, the orbit projection, primitive
//! assembly and both fill algorithms unchanged. After coverage it computes
//! interpolation weights at the same pixel centers, builds a `SurfaceSample`
//! per fragment, evaluates a shader closure on scoped standard threads and
//! writes the colors through the depth-tested `Framebuffer::point_depth`.

use crate::geometry::Vertex;
use crate::pipeline::{
    NormalSource, OrbitView, Projection, RenderError, RenderSettings, SurfaceSample, Uniforms,
    face_normal, primitive_assembly, project_orbit, render_with_settings, vertex_shader,
};
use crate::rasterization::coverage::{interpolation_weights, vertex_point};
use crate::rasterization::{RasterBounds, TriangleMode, fill_barycentric, fill_scanline};
use crate::rendering::Framebuffer;
use raylib::math::Vector3;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Options of the shaded stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadedOptions {
    /// Skip triangles facing away from the eye (closed meshes such as the
    /// course sphere then cover every pixel exactly once).
    pub cull_back_faces: bool,
    pub normal_source: NormalSource,
    /// Shader threads; 0 uses `available_parallelism`.
    pub threads: usize,
}

impl Default for ShadedOptions {
    fn default() -> Self {
        Self {
            cull_back_faces: true,
            normal_source: NormalSource::Radial,
            threads: 0,
        }
    }
}

/// Real counts and timings of one shaded render.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShadedStats {
    pub vertices_processed: usize,
    pub triangles_processed: usize,
    pub triangles_culled: usize,
    pub fragments_generated: usize,
    pub fragments_shaded: usize,
    pub raster_time: Duration,
    pub shade_time: Duration,
}

/// Renders `vertex_array` with the orbit `view`, shading every visible
/// fragment with `shader`. `Wireframe` delegates to the laboratory 4/5
/// outline path (no shader evaluation).
pub fn render_shaded<S>(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    view: OrbitView,
    mode: TriangleMode,
    options: ShadedOptions,
    shader: &S,
) -> Result<ShadedStats, RenderError>
where
    S: Fn(&SurfaceSample) -> Vector3 + Sync,
{
    if mode == TriangleMode::Wireframe {
        let s = render_with_settings(
            framebuffer,
            uniforms,
            vertex_array,
            Projection::Orbit(view),
            TriangleMode::Wireframe,
            RenderSettings::default(),
        )?;
        return Ok(ShadedStats {
            vertices_processed: s.vertices_processed,
            triangles_processed: s.triangles_processed,
            fragments_generated: s.fragments_generated,
            ..ShadedStats::default()
        });
    }

    let raster_start = Instant::now();
    let mut transformed: Vec<Vertex> = vertex_array
        .iter()
        .map(|v| vertex_shader(v, uniforms))
        .collect();
    let world: Vec<Vector3> = transformed.iter().map(|v| v.transformed_position).collect();
    project_orbit(
        &mut transformed,
        &view,
        framebuffer.width(),
        framebuffer.height(),
    )
    .map_err(RenderError::Mapping)?;
    let triangles = primitive_assembly(&transformed).map_err(RenderError::Primitive)?;
    let bounds = RasterBounds::new(framebuffer.width(), framebuffer.height());
    let eye = view.camera.eye_direction();

    let mut stats = ShadedStats {
        vertices_processed: transformed.len(),
        triangles_processed: triangles.len(),
        ..ShadedStats::default()
    };
    let mut samples: Vec<SurfaceSample> = Vec::new();
    for (i, tri) in triangles.iter().enumerate() {
        let k = 3 * i;
        let Some(world_normal) = face_normal(world[k], world[k + 1], world[k + 2]) else {
            stats.triangles_culled += 1;
            continue;
        };
        if options.cull_back_faces && world_normal.dot(eye) <= 0.0 {
            stats.triangles_culled += 1;
            continue;
        }
        let fragments = match mode {
            TriangleMode::FilledBarycentric => fill_barycentric(&tri[0], &tri[1], &tri[2], bounds),
            _ => fill_scanline(&tri[0], &tri[1], &tri[2], bounds),
        };
        stats.fragments_generated += fragments.len();
        let screen = [
            vertex_point(&tri[0]),
            vertex_point(&tri[1]),
            vertex_point(&tri[2]),
        ];
        let object = [
            vertex_array[k].position,
            vertex_array[k + 1].position,
            vertex_array[k + 2].position,
        ];
        let object_normal = face_normal(object[0], object[1], object[2]).unwrap_or(world_normal);
        for fragment in &fragments {
            let (x, y) = (fragment.position.x as i32, fragment.position.y as i32);
            // Early depth rejection against geometry already in the buffer.
            if let Some(stored) = framebuffer.get_depth(x, y)
                && fragment.depth >= stored
            {
                continue;
            }
            let center = (x as f64 + 0.5, y as f64 + 0.5);
            let Some(w) = interpolation_weights(screen[0], screen[1], screen[2], center) else {
                continue;
            };
            let p = object[0] * w[0] as f32 + object[1] * w[1] as f32 + object[2] * w[2] as f32;
            let normal = match options.normal_source {
                NormalSource::Radial => {
                    let len = p.length();
                    if len.is_finite() && len > 0.0 {
                        p / len
                    } else {
                        object_normal
                    }
                }
                NormalSource::Face => object_normal,
            };
            samples.push(SurfaceSample {
                x,
                y,
                depth: fragment.depth,
                object_position: p,
                normal,
                face_normal: object_normal,
            });
        }
    }
    stats.raster_time = raster_start.elapsed();

    let shade_start = Instant::now();
    let colors = shade_all(&samples, shader, options.threads);
    stats.shade_time = shade_start.elapsed();
    stats.fragments_shaded = samples.len();

    for (s, color) in samples.iter().zip(colors) {
        framebuffer.point_depth(s.x, s.y, color, s.depth);
    }
    Ok(stats)
}

/// Fragments per job of the shader threads.
const SHADE_CHUNK: usize = 512;

/// Evaluates the shader on small chunks that idle threads take from a shared
/// queue, so fast and slow cores (and cheap and expensive regions) stay
/// busy. Every color is written at its sample's index, so the result does
/// not depend on the number of threads.
fn shade_all<S>(samples: &[SurfaceSample], shader: &S, threads: usize) -> Vec<Vector3>
where
    S: Fn(&SurfaceSample) -> Vector3 + Sync,
{
    let n = match threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    };
    if n <= 1 || samples.len() < 2 {
        return samples.iter().map(|s| sanitize(shader(s))).collect();
    }
    let mut colors = vec![Vector3::zero(); samples.len()];
    let jobs = Mutex::new(
        samples
            .chunks(SHADE_CHUNK)
            .zip(colors.chunks_mut(SHADE_CHUNK)),
    );
    std::thread::scope(|scope| {
        for _ in 0..n.min(samples.len().div_ceil(SHADE_CHUNK)) {
            scope.spawn(|| {
                loop {
                    let job = jobs.lock().expect("shader job queue").next();
                    let Some((part, out)) = job else { break };
                    for (s, c) in part.iter().zip(out) {
                        *c = sanitize(shader(s));
                    }
                }
            });
        }
    });
    colors
}

/// Non-finite components become 0 so a shader bug cannot poison the image.
fn sanitize(c: Vector3) -> Vector3 {
    let f = |v: f32| if v.is_finite() { v } else { 0.0 };
    Vector3::new(f(c.x), f(c.y), f(c.z))
}

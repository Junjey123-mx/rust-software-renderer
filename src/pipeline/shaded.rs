//! Laboratory 6: programmable fragment stage on the existing pipeline.
//!
//! `render_shaded` reuses the vertex shader, the orbit projection, primitive
//! assembly and both fill algorithms unchanged. After coverage it computes
//! interpolation weights at the same pixel centers, builds a `SurfaceSample`
//! per fragment, evaluates a shader closure on scoped standard threads and
//! writes the colors through the depth-tested `Framebuffer::point_depth`.
//!
//! `render_shaded_with` adds a perspective path beside it (Camino al
//! Proyecto 3): triangles are clipped against the near plane in view space,
//! projected, rasterized with the same fills, interpolated with
//! perspective-correct weights and shaded with a per-fragment direction
//! towards the eye. The orbit path is the laboratory six code, bit for bit.

use crate::geometry::Vertex;
use crate::pipeline::{
    ClipVertex, NormalSource, OrbitView, PerspectiveCamera, Projection, RenderError,
    RenderSettings, SurfaceSample, Uniforms, clip_near, face_normal, primitive_assembly,
    project_orbit, render_with_settings, vertex_shader,
};
use crate::rasterization::coverage::{interpolation_weights, vertex_point};
use crate::rasterization::{RasterBounds, TriangleMode, fill_barycentric, fill_scanline, line};
use crate::rendering::Framebuffer;
use raylib::math::{Vector2, Vector3};
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

/// How world positions become pixels in the shaded stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewProjection {
    /// Orthographic orbit camera (laboratories 5 and 6).
    Orbit(OrbitView),
    /// Perspective camera with near-plane clipping.
    Perspective(PerspectiveCamera),
}

/// View-dependent data of one fragment, handed to the shader next to its
/// `SurfaceSample`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentView {
    /// Unit direction from the fragment towards the eye, world space. With
    /// the orbit camera it is the same for every fragment.
    pub view_direction: Vector3,
}

/// Per-frame buffers of the shaded stage, owned by the caller so their
/// capacity survives between draws instead of being reallocated for every
/// instance. Contents never carry over: every draw clears them first.
///
/// The same buffers also serve a frame batch: every instance of a frame is
/// rasterized first with its depth written at once (`rasterize`), all
/// samples are shaded by one thread pool (`shade_batch`) and the colors are
/// written where each sample won the depth test (`resolve`). The image is
/// identical to drawing the instances one by one.
#[derive(Debug, Clone, Default)]
pub struct RenderScratch {
    samples: Vec<SurfaceSample>,
    views: Vec<FragmentView>,
    /// Batch tag (instance index) of every sample.
    tags: Vec<u32>,
    colors: Vec<Vector3>,
}

impl RenderScratch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fragments the buffers can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.samples.capacity().min(self.colors.capacity())
    }

    fn clear(&mut self) {
        self.samples.clear();
        self.views.clear();
        self.tags.clear();
    }

    fn sink(&mut self, tag: u32, write_depth: bool) -> Sink<'_> {
        Sink {
            samples: &mut self.samples,
            views: &mut self.views,
            tags: &mut self.tags,
            tag,
            write_depth,
        }
    }

    /// Starts a frame batch.
    pub fn begin_batch(&mut self) {
        self.clear();
    }

    /// Samples collected by the current batch.
    pub fn batch_len(&self) -> usize {
        self.samples.len()
    }

    /// Rasterizes one instance into the batch with `tag`, writing the depth
    /// of every accepted sample at once. Filled modes collect samples; the
    /// wireframe outline is drawn immediately and adds none.
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize(
        &mut self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        vertex_array: &[Vertex],
        projection: &ViewProjection,
        mode: TriangleMode,
        options: ShadedOptions,
        tag: u32,
    ) -> Result<ShadedStats, RenderError> {
        match projection {
            ViewProjection::Orbit(view) => {
                if mode == TriangleMode::Wireframe {
                    return orbit_wireframe(framebuffer, uniforms, vertex_array, *view);
                }
                let view_direction = view.camera.eye_direction();
                let mut sink = self.sink(tag, true);
                raster_orbit(
                    framebuffer,
                    uniforms,
                    vertex_array,
                    *view,
                    mode,
                    options,
                    view_direction,
                    &mut sink,
                )
            }
            ViewProjection::Perspective(camera) => {
                let mut sink = self.sink(tag, true);
                raster_perspective(
                    framebuffer,
                    uniforms,
                    vertex_array,
                    camera,
                    mode,
                    options,
                    &mut sink,
                )
            }
        }
    }

    /// Shades every sample of the batch with one thread pool; `shader`
    /// receives the sample's tag.
    pub fn shade_batch<F>(&mut self, threads: usize, shader: F)
    where
        F: Fn(u32, &SurfaceSample, &FragmentView) -> Vector3 + Sync,
    {
        let (samples, views, tags) = (&self.samples, &self.views, &self.tags);
        shade_indexed(&mut self.colors, samples.len(), threads, |i| {
            shader(tags[i], &samples[i], &views[i])
        });
    }

    /// Writes each shaded sample where it won the depth test of the batch.
    pub fn resolve(&self, framebuffer: &mut Framebuffer) {
        for (s, &color) in self.samples.iter().zip(&self.colors) {
            framebuffer.point_if_depth(s.x, s.y, color, s.depth);
        }
    }
}

/// Destination of rasterized samples.
struct Sink<'a> {
    samples: &'a mut Vec<SurfaceSample>,
    views: &'a mut Vec<FragmentView>,
    tags: &'a mut Vec<u32>,
    tag: u32,
    /// Frame batches write each accepted depth at once so later instances
    /// are rejected early; single draws resolve depth when writing color.
    write_depth: bool,
}

impl Sink<'_> {
    fn push(&mut self, framebuffer: &mut Framebuffer, sample: SurfaceSample, view: FragmentView) {
        if self.write_depth {
            framebuffer.set_depth(sample.x, sample.y, sample.depth);
        }
        self.samples.push(sample);
        self.views.push(view);
        self.tags.push(self.tag);
    }
}

/// Laboratory 4/5 outline of the orbit path (no shader evaluation).
fn orbit_wireframe(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    view: OrbitView,
) -> Result<ShadedStats, RenderError> {
    let s = render_with_settings(
        framebuffer,
        uniforms,
        vertex_array,
        Projection::Orbit(view),
        TriangleMode::Wireframe,
        RenderSettings::default(),
    )?;
    Ok(ShadedStats {
        vertices_processed: s.vertices_processed,
        triangles_processed: s.triangles_processed,
        fragments_generated: s.fragments_generated,
        ..ShadedStats::default()
    })
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
    render_orbit(
        framebuffer,
        uniforms,
        vertex_array,
        view,
        mode,
        options,
        shader,
        &mut RenderScratch::new(),
    )
}

#[allow(clippy::too_many_arguments)]
fn render_orbit<S>(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    view: OrbitView,
    mode: TriangleMode,
    options: ShadedOptions,
    shader: &S,
    scratch: &mut RenderScratch,
) -> Result<ShadedStats, RenderError>
where
    S: Fn(&SurfaceSample) -> Vector3 + Sync,
{
    if mode == TriangleMode::Wireframe {
        return orbit_wireframe(framebuffer, uniforms, vertex_array, view);
    }
    scratch.clear();
    let view_direction = view.camera.eye_direction();
    let mut stats = raster_orbit(
        framebuffer,
        uniforms,
        vertex_array,
        view,
        mode,
        options,
        view_direction,
        &mut scratch.sink(0, false),
    )?;

    let shade_start = Instant::now();
    let samples = &scratch.samples;
    shade_indexed(&mut scratch.colors, samples.len(), options.threads, |i| {
        shader(&samples[i])
    });
    stats.shade_time = shade_start.elapsed();
    stats.fragments_shaded = samples.len();

    for (s, &color) in samples.iter().zip(&scratch.colors) {
        framebuffer.point_depth(s.x, s.y, color, s.depth);
    }
    Ok(stats)
}

/// Filled orbit rasterization: vertex shader, orbit projection, assembly,
/// back-face culling with the constant eye, coverage, early depth rejection
/// and affine interpolation of the object position at pixel centers.
#[allow(clippy::too_many_arguments)]
fn raster_orbit(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    view: OrbitView,
    mode: TriangleMode,
    options: ShadedOptions,
    view_direction: Vector3,
    sink: &mut Sink<'_>,
) -> Result<ShadedStats, RenderError> {
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
    let fragment_view = FragmentView { view_direction };

    let mut stats = ShadedStats {
        vertices_processed: transformed.len(),
        triangles_processed: triangles.len(),
        ..ShadedStats::default()
    };
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
            let sample = SurfaceSample {
                x,
                y,
                depth: fragment.depth,
                object_position: p,
                normal,
                face_normal: object_normal,
            };
            sink.push(framebuffer, sample, fragment_view);
        }
    }
    stats.raster_time = raster_start.elapsed();
    Ok(stats)
}

/// Renders through either projection. With `Orbit` this is exactly
/// `render_shaded` (the view direction handed to the shader is the constant
/// eye direction); with `Perspective` it is the clipped, perspective-correct
/// path with a per-fragment view direction.
pub fn render_shaded_with<S>(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: &ViewProjection,
    mode: TriangleMode,
    options: ShadedOptions,
    shader: &S,
) -> Result<ShadedStats, RenderError>
where
    S: Fn(&SurfaceSample, &FragmentView) -> Vector3 + Sync,
{
    render_shaded_with_scratch(
        framebuffer,
        uniforms,
        vertex_array,
        projection,
        mode,
        options,
        shader,
        &mut RenderScratch::new(),
    )
}

/// `render_shaded_with` reusing the caller's buffers.
#[allow(clippy::too_many_arguments)]
pub fn render_shaded_with_scratch<S>(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    projection: &ViewProjection,
    mode: TriangleMode,
    options: ShadedOptions,
    shader: &S,
    scratch: &mut RenderScratch,
) -> Result<ShadedStats, RenderError>
where
    S: Fn(&SurfaceSample, &FragmentView) -> Vector3 + Sync,
{
    match projection {
        ViewProjection::Orbit(view) => {
            let fragment_view = FragmentView {
                view_direction: view.camera.eye_direction(),
            };
            render_orbit(
                framebuffer,
                uniforms,
                vertex_array,
                *view,
                mode,
                options,
                &|s| shader(s, &fragment_view),
                scratch,
            )
        }
        ViewProjection::Perspective(camera) => render_perspective(
            framebuffer,
            uniforms,
            vertex_array,
            camera,
            mode,
            options,
            shader,
            scratch,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_perspective<S>(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    camera: &PerspectiveCamera,
    mode: TriangleMode,
    options: ShadedOptions,
    shader: &S,
    scratch: &mut RenderScratch,
) -> Result<ShadedStats, RenderError>
where
    S: Fn(&SurfaceSample, &FragmentView) -> Vector3 + Sync,
{
    scratch.clear();
    let mut stats = raster_perspective(
        framebuffer,
        uniforms,
        vertex_array,
        camera,
        mode,
        options,
        &mut scratch.sink(0, false),
    )?;

    let shade_start = Instant::now();
    let (samples, views) = (&scratch.samples, &scratch.views);
    shade_indexed(&mut scratch.colors, samples.len(), options.threads, |i| {
        shader(&samples[i], &views[i])
    });
    stats.shade_time = shade_start.elapsed();
    stats.fragments_shaded = samples.len();

    for (s, &color) in samples.iter().zip(&scratch.colors) {
        framebuffer.point_depth(s.x, s.y, color, s.depth);
    }
    Ok(stats)
}

/// Perspective rasterization: vertex shader, view transform, back-face
/// culling with the per-triangle view vector, near-plane clipping with full
/// attributes, coverage, perspective-correct weights and depth, early depth
/// rejection and a per-fragment view direction. `Wireframe` draws the
/// clipped outlines directly and collects no sample.
fn raster_perspective(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    camera: &PerspectiveCamera,
    mode: TriangleMode,
    options: ShadedOptions,
    sink: &mut Sink<'_>,
) -> Result<ShadedStats, RenderError> {
    let raster_start = Instant::now();
    let (width, height) = (framebuffer.width(), framebuffer.height());
    let corners: Vec<ClipVertex> = vertex_array
        .iter()
        .map(|v| ClipVertex::from_world(&vertex_shader(v, uniforms), camera))
        .collect();
    let triangles = primitive_assembly(vertex_array).map_err(RenderError::Primitive)?;
    let bounds = RasterBounds::new(width, height);

    let mut stats = ShadedStats {
        vertices_processed: corners.len(),
        triangles_processed: triangles.len(),
        ..ShadedStats::default()
    };
    for i in 0..triangles.len() {
        let tri = [corners[3 * i], corners[3 * i + 1], corners[3 * i + 2]];
        let Some(world_normal) = face_normal(
            tri[0].world_position,
            tri[1].world_position,
            tri[2].world_position,
        ) else {
            stats.triangles_culled += 1;
            continue;
        };
        if options.cull_back_faces {
            let centroid =
                (tri[0].world_position + tri[1].world_position + tri[2].world_position) / 3.0;
            if world_normal.dot(camera.eye - centroid) <= 0.0 {
                stats.triangles_culled += 1;
                continue;
            }
        }
        let object_normal = face_normal(
            tri[0].object_position,
            tri[1].object_position,
            tri[2].object_position,
        )
        .unwrap_or(world_normal);
        let polygon = clip_near(&tri, camera.near);
        if polygon.is_empty() {
            stats.triangles_culled += 1;
            continue;
        }
        if mode == TriangleMode::Wireframe {
            // Outline of the clipped polygon: no fan diagonal, no depth.
            for (a, b) in polygon.edges() {
                let (Some(pa), Some(pb)) = (
                    a.to_screen(camera, width, height),
                    b.to_screen(camera, width, height),
                ) else {
                    continue;
                };
                let fragments = line(
                    Vector2::new(pa.transformed_position.x, pa.transformed_position.y),
                    Vector2::new(pb.transformed_position.x, pb.transformed_position.y),
                    pa.color,
                );
                stats.fragments_generated += fragments.len();
                for f in &fragments {
                    framebuffer.point(f.position.x as i32, f.position.y as i32, f.color);
                }
            }
            continue;
        }
        for clipped in polygon.triangles() {
            let (Some(a), Some(b), Some(c)) = (
                clipped[0].to_screen(camera, width, height),
                clipped[1].to_screen(camera, width, height),
                clipped[2].to_screen(camera, width, height),
            ) else {
                continue;
            };
            let fragments = match mode {
                TriangleMode::FilledBarycentric => fill_barycentric(&a, &b, &c, bounds),
                _ => fill_scanline(&a, &b, &c, bounds),
            };
            stats.fragments_generated += fragments.len();
            let screen = [vertex_point(&a), vertex_point(&b), vertex_point(&c)];
            let z = [
                clipped[0].view_position.z as f64,
                clipped[1].view_position.z as f64,
                clipped[2].view_position.z as f64,
            ];
            for fragment in &fragments {
                let (x, y) = (fragment.position.x as i32, fragment.position.y as i32);
                let center = (x as f64 + 0.5, y as f64 + 0.5);
                let Some(l) = interpolation_weights(screen[0], screen[1], screen[2], center) else {
                    continue;
                };
                // Perspective-correct weights: l_i / z_i normalised; the
                // depth is the reciprocal of the normaliser.
                let q = [l[0] / z[0], l[1] / z[1], l[2] / z[2]];
                let d = q[0] + q[1] + q[2];
                if !(d.is_finite() && d > 0.0) {
                    continue;
                }
                let depth = (1.0 / d) as f32;
                if let Some(stored) = framebuffer.get_depth(x, y)
                    && depth >= stored
                {
                    continue;
                }
                let w = [(q[0] / d) as f32, (q[1] / d) as f32, (q[2] / d) as f32];
                let p = clipped[0].object_position * w[0]
                    + clipped[1].object_position * w[1]
                    + clipped[2].object_position * w[2];
                let world = clipped[0].world_position * w[0]
                    + clipped[1].world_position * w[1]
                    + clipped[2].world_position * w[2];
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
                let sample = SurfaceSample {
                    x,
                    y,
                    depth,
                    object_position: p,
                    normal,
                    face_normal: object_normal,
                };
                let view = FragmentView {
                    view_direction: camera.view_direction(world),
                };
                sink.push(framebuffer, sample, view);
            }
        }
    }
    stats.raster_time = raster_start.elapsed();
    Ok(stats)
}

/// Fragments per job of the shader threads.
const SHADE_CHUNK: usize = 512;

/// Evaluates `color(i)` for `0..count` into `out` (resized, capacity
/// kept) on small chunks that idle threads take from a shared queue, so fast
/// and slow cores (and cheap and expensive regions) stay busy. Every color
/// is written at its sample's index, so the result does not depend on the
/// number of threads; sequential below two samples or with one thread.
fn shade_indexed<F>(out: &mut Vec<Vector3>, count: usize, threads: usize, color: F)
where
    F: Fn(usize) -> Vector3 + Sync,
{
    out.clear();
    out.resize(count, Vector3::zero());
    let n = match threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    };
    if n <= 1 || count < 2 {
        for (i, c) in out.iter_mut().enumerate() {
            *c = sanitize(color(i));
        }
        return;
    }
    let jobs = Mutex::new(
        (0..count)
            .step_by(SHADE_CHUNK)
            .zip(out.chunks_mut(SHADE_CHUNK)),
    );
    std::thread::scope(|scope| {
        for _ in 0..n.min(count.div_ceil(SHADE_CHUNK)) {
            scope.spawn(|| {
                loop {
                    let job = jobs.lock().expect("shader job queue").next();
                    let Some((start, chunk)) = job else { break };
                    for (offset, c) in chunk.iter_mut().enumerate() {
                        *c = sanitize(color(start + offset));
                    }
                }
            });
        }
    });
}

/// Non-finite components become 0 so a shader bug cannot poison the image.
fn sanitize(c: Vector3) -> Vector3 {
    let f = |v: f32| if v.is_finite() { v } else { 0.0 };
    Vector3::new(f(c.x), f(c.y), f(c.z))
}

//! The programmable shaded stage on the course sphere.

use raylib::math::Vector3;
use rust_software_renderer::config::SPHERE_MODEL_PATH;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    NormalSource, OrbitCamera, OrbitView, Projection, RenderSettings, ShadedOptions, SurfaceSample,
    Uniforms, ViewPreset, Viewport, model_bounds, render_interactive, render_shaded,
    render_with_settings,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;
use std::path::Path;

const W: u32 = 200;
const H: u32 = 150;

fn sphere() -> Vec<Vertex> {
    load_obj(Path::new(SPHERE_MODEL_PATH))
        .unwrap()
        .expand_vertices(Vertex::DEFAULT_COLOR)
}

fn view(vertices: &[Vertex]) -> OrbitView {
    let bounds = model_bounds(vertices).unwrap();
    let cam = OrbitCamera::new(bounds.center, ViewPreset::Oblique);
    OrbitView::new(cam, &bounds, Viewport::new(W, H, 0.1)).unwrap()
}

fn normal_color(s: &SurfaceSample) -> Vector3 {
    s.normal * 0.5 + Vector3::new(0.5, 0.5, 0.5)
}

fn shaded(mode: TriangleMode, options: ShadedOptions) -> (Framebuffer, usize, usize) {
    let v = sphere();
    let mut fb = Framebuffer::new(W, H);
    let stats = render_shaded(
        &mut fb,
        &Uniforms::default(),
        &v,
        view(&v),
        mode,
        options,
        &normal_color,
    )
    .unwrap();
    (fb, stats.fragments_shaded, stats.triangles_culled)
}

fn mask(fb: &Framebuffer) -> Vec<bool> {
    let bg = fb.background_color();
    (0..H as i32)
        .flat_map(|y| (0..W as i32).map(move |x| (x, y)))
        .map(|(x, y)| fb.get_pixel(x, y).unwrap() != bg)
        .collect()
}

#[test]
fn shaded_coverage_matches_interactive_coverage() {
    let v = sphere();
    let mut reference = Framebuffer::new(W, H);
    render_interactive(
        &mut reference,
        &Uniforms::default(),
        &v,
        view(&v),
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    let (fb, _, _) = shaded(TriangleMode::FilledBarycentric, ShadedOptions::default());
    assert_eq!(mask(&fb), mask(&reference));
    assert!(fb.count_non_background() > 10_000);
}

#[test]
fn barycentric_and_scanline_shaded_images_are_identical() {
    let (a, _, _) = shaded(TriangleMode::FilledBarycentric, ShadedOptions::default());
    let (b, _, _) = shaded(TriangleMode::FilledScanline, ShadedOptions::default());
    assert_eq!(a.to_rgba_bytes(), b.to_rgba_bytes());
}

#[test]
fn back_face_culling_shades_each_pixel_once() {
    let (fb, shaded_count, culled) = shaded(TriangleMode::FilledScanline, ShadedOptions::default());
    assert_eq!(shaded_count, fb.count_non_background());
    assert_eq!(culled, 96);
}

#[test]
fn thread_count_does_not_change_the_image() {
    let images: Vec<Vec<u8>> = [1, 3, 0]
        .into_iter()
        .map(|threads| {
            let options = ShadedOptions {
                threads,
                ..ShadedOptions::default()
            };
            shaded(TriangleMode::FilledScanline, options)
                .0
                .to_rgba_bytes()
        })
        .collect();
    assert_eq!(images[0], images[1]);
    assert_eq!(images[0], images[2]);
}

#[test]
fn shaded_render_writes_the_depth_buffer() {
    let (fb, _, _) = shaded(TriangleMode::FilledScanline, ShadedOptions::default());
    assert!(
        fb.get_depth(W as i32 / 2, H as i32 / 2)
            .unwrap()
            .is_finite()
    );
    assert_eq!(fb.get_depth(0, 0).unwrap(), f32::INFINITY);
}

#[test]
fn nearer_geometry_drawn_first_hides_the_sphere() {
    let v = sphere();
    let view = view(&v);
    let b = view.camera.basis();
    let c = view.camera.eye_direction() * 2.0;
    let red = Vector3::new(1.0, 0.0, 0.0);
    let blocker: Vec<Vertex> = [
        c - b.right * 0.3 - b.up * 0.3,
        c + b.right * 0.3 - b.up * 0.3,
        c + b.up * 0.3,
    ]
    .into_iter()
    .map(|p| Vertex::new_with_color(p, red))
    .collect();
    let mut fb = Framebuffer::new(W, H);
    render_with_settings(
        &mut fb,
        &Uniforms::default(),
        &blocker,
        Projection::Orbit(view),
        TriangleMode::FilledScanline,
        RenderSettings {
            depth_test: true,
            shading: None,
        },
    )
    .unwrap();
    render_shaded(
        &mut fb,
        &Uniforms::default(),
        &v,
        view,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
        &normal_color,
    )
    .unwrap();
    assert_eq!(fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap(), red);
    // Outside the blocker the sphere is still visible.
    assert_ne!(
        fb.get_pixel(W as i32 / 2, H as i32 / 2 + 50).unwrap(),
        fb.background_color()
    );
}

#[test]
fn radial_normal_faces_the_eye_at_the_disk_center() {
    let v = sphere();
    let view = view(&v);
    let eye = view.camera.eye_direction();
    let mut fb = Framebuffer::new(W, H);
    render_shaded(
        &mut fb,
        &Uniforms::default(),
        &v,
        view,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
        &|s: &SurfaceSample| Vector3::new(s.normal.dot(eye), 0.0, 0.0),
    )
    .unwrap();
    assert!(fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap().x > 0.99);
}

#[test]
fn wireframe_mode_delegates_to_the_outline_path() {
    let v = sphere();
    let (fb, shaded_count, _) = shaded(TriangleMode::Wireframe, ShadedOptions::default());
    let mut reference = Framebuffer::new(W, H);
    render_with_settings(
        &mut reference,
        &Uniforms::default(),
        &v,
        Projection::Orbit(view(&v)),
        TriangleMode::Wireframe,
        RenderSettings::default(),
    )
    .unwrap();
    assert_eq!(fb.to_rgba_bytes(), reference.to_rgba_bytes());
    assert_eq!(shaded_count, 0);
}

#[test]
fn non_finite_shader_output_is_sanitized() {
    let v = sphere();
    let mut fb = Framebuffer::new(W, H);
    render_shaded(
        &mut fb,
        &Uniforms::default(),
        &v,
        view(&v),
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
        &|_: &SurfaceSample| Vector3::new(f32::NAN, 0.5, f32::INFINITY),
    )
    .unwrap();
    assert_eq!(
        fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap(),
        Vector3::new(0.0, 0.5, 0.0)
    );
}

#[test]
fn face_normal_source_is_flat_per_triangle() {
    let options = ShadedOptions {
        normal_source: NormalSource::Face,
        ..ShadedOptions::default()
    };
    let (fb, _, _) = shaded(TriangleMode::FilledScanline, options);
    let bg = fb.background_color();
    let colors: HashSet<[u32; 3]> = (0..H as i32)
        .flat_map(|y| (0..W as i32).map(move |x| (x, y)))
        .map(|(x, y)| fb.get_pixel(x, y).unwrap())
        .filter(|&c| c != bg)
        .map(|c| [c.x.to_bits(), c.y.to_bits(), c.z.to_bits()])
        .collect();
    assert!(!colors.is_empty() && colors.len() <= 96, "{}", colors.len());
}

#[test]
fn many_shading_chunks_keep_the_image_independent_of_threads() {
    use rust_software_renderer::celestial::{Body, CelestialScene};
    use rust_software_renderer::pipeline::ViewPreset;
    // The rocky planet has very uneven per-fragment cost and this size
    // gives dozens of shading chunks to the work queue.
    let render = |threads: usize| {
        let mut scene =
            CelestialScene::new(Body::CrimsonEntrance, ViewPreset::Oblique, 320, 240).unwrap();
        scene.time = 8.0;
        let mut fb = Framebuffer::new(320, 240);
        let stats = scene
            .render(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions {
                    threads,
                    ..ShadedOptions::default()
                },
            )
            .unwrap();
        assert!(stats.fragments_shaded > 20 * 512);
        fb.to_rgba_bytes()
    };
    let one = render(1);
    assert_eq!(one, render(7));
    assert_eq!(one, render(20));
}

//! Laboratory 6 regression guard: frozen outputs of laboratories 4 and 5.
//!
//! Each case renders the spacecraft or the triangle scene exactly as the
//! graded laboratory five tree did and compares the fragment count and an
//! FNV-1a fingerprint of the RGBA bytes. The values were captured before any
//! laboratory six change, so a single changed pixel fails the suite.

use rust_software_renderer::config::AppConfig;
use rust_software_renderer::geometry::{Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, Projection, Uniforms, ViewPlane, ViewPreset, Viewport, model_bounds,
    render_interactive, render_with_mode,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;

const MODES: [TriangleMode; 3] = TriangleMode::ALL;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fingerprint(fb: &Framebuffer) -> u64 {
    fnv1a64(&fb.to_rgba_bytes())
}

fn ship() -> Vec<Vertex> {
    load_obj(&AppConfig::default().model_path)
        .expect("ship model loads")
        .expand_vertices(Vertex::DEFAULT_COLOR)
}

fn fit(mode: TriangleMode) -> (usize, u64) {
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &ship(),
        Projection::Fit {
            plane: ViewPlane::XZ,
            margin_ratio: 0.1,
        },
        mode,
    )
    .unwrap();
    (stats.fragments_generated, fingerprint(&fb))
}

fn triangles(mode: TriangleMode) -> (usize, u64) {
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &triangle_scene(),
        Projection::Screen,
        mode,
    )
    .unwrap();
    (stats.fragments_generated, fingerprint(&fb))
}

fn orbit(mode: TriangleMode, preset: ViewPreset, zoom: f32) -> (usize, u64) {
    let ship = ship();
    let bounds = model_bounds(&ship).unwrap();
    let mut cam = OrbitCamera::new(bounds.center, preset);
    cam.zoom_by(zoom);
    let view = OrbitView::new(cam, &bounds, Viewport::new(800, 600, 0.1)).unwrap();
    let mut fb = Framebuffer::new(800, 600);
    let stats = render_interactive(&mut fb, &Uniforms::default(), &ship, view, mode).unwrap();
    (stats.fragments_generated, fingerprint(&fb))
}

fn check(label: &str, actual: &[(usize, u64)], expected: &[(usize, u64)]) {
    let printed: Vec<String> = actual
        .iter()
        .map(|(n, h)| format!("({n}, {h:#018x})"))
        .collect();
    for &(n, h) in expected {
        assert_ne!(h, 0, "{label}: hash not captured yet; actual = {printed:?}");
        assert_ne!(
            n, 0,
            "{label}: count not captured yet; actual = {printed:?}"
        );
    }
    assert_eq!(actual, expected, "{label}: actual = {printed:?}");
}

const FIT: [(usize, u64); 3] = [
    (79934, 0xf96077b19acbcd4c),
    (173500, 0xf5782b4fdcbc462d),
    (173500, 0xf5782b4fdcbc462d),
];
const TRIANGLES: [(usize, u64); 3] = [
    (4831, 0x1469cd5e9b9ded1b),
    (192132, 0x1a4adbd7fb1b3a06),
    (192132, 0x1a4adbd7fb1b3a06),
];
/// Mode-major: wireframe Oblique, Top, Front, Side, then barycentric, scanline.
const ORBIT: [(usize, u64); 12] = [
    (51180, 0xe67e33154a4d812c),
    (62620, 0xb3914b1bd49c08cd),
    (31966, 0xd41c9e8b4279d324),
    (58736, 0x125c722af8d9c7fc),
    (96270, 0x7f189c8fbf96b261),
    (104976, 0x603a96406ebab916),
    (23376, 0xccae3acdcb2e027f),
    (74414, 0x77d38b83f16c8aa9),
    (96270, 0x7f189c8fbf96b261),
    (104976, 0x603a96406ebab916),
    (23376, 0xccae3acdcb2e027f),
    (74414, 0x77d38b83f16c8aa9),
];
const ORBIT_ZOOMED: [(usize, u64); 3] = [
    (100048, 0x89058ab6f996978d),
    (358706, 0x0b332783930f5bd3),
    (358706, 0x0b332783930f5bd3),
];

#[test]
fn lab4_fit_outputs_are_frozen() {
    let actual: Vec<_> = MODES.iter().map(|&m| fit(m)).collect();
    assert_eq!(actual[0].0, 79_934, "Fit wireframe fragment count");
    check("fit", &actual, &FIT);
}

#[test]
fn lab5_triangle_scene_outputs_are_frozen() {
    let actual: Vec<_> = MODES.iter().map(|&m| triangles(m)).collect();
    assert_eq!(
        actual[2].0, 192_132,
        "triangle scene scanline fragment count"
    );
    check("triangles", &actual, &TRIANGLES);
}

#[test]
fn interactive_ship_outputs_are_frozen() {
    let presets = [
        ViewPreset::Oblique,
        ViewPreset::Top,
        ViewPreset::Front,
        ViewPreset::Side,
    ];
    let actual: Vec<_> = MODES
        .iter()
        .flat_map(|&m| presets.iter().map(move |&p| orbit(m, p, 1.0)))
        .collect();
    check("orbit", &actual, &ORBIT);
}

#[test]
fn interactive_zoomed_ship_outputs_are_frozen() {
    let actual: Vec<_> = MODES
        .iter()
        .map(|&m| orbit(m, ViewPreset::Oblique, 2.0))
        .collect();
    check("orbit zoomed", &actual, &ORBIT_ZOOMED);
}

use raylib::math::Vector3;
use rust_software_renderer::app::{CameraAction, USAGE, apply_input, parse_args};
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, Uniforms, ViewPreset, Viewport, model_bounds, render_interactive,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

fn args(list: &[&str]) -> Result<rust_software_renderer::app::CliArgs, String> {
    parse_args(list.iter().map(|s| s.to_string()))
}

#[test]
fn view_flag_parses_every_preset_and_rejects_others() {
    assert_eq!(args(&[]).unwrap().view, None);
    for (name, preset) in [
        ("oblique", ViewPreset::Oblique),
        ("top", ViewPreset::Top),
        ("front", ViewPreset::Front),
        ("side", ViewPreset::Side),
    ] {
        let a = args(&["--view", name, "--mode", "scanline"]).unwrap();
        assert_eq!(a.view, Some(preset));
        assert_eq!(a.mode, TriangleMode::FilledScanline);
    }
    assert!(args(&["--view"]).is_err());
    assert!(args(&["--view", "back"]).unwrap_err().contains("back"));
    assert!(args(&["--help"]).unwrap().help && args(&["-h"]).unwrap().help);
    assert!(USAGE.contains("--view") && USAGE.contains("1/2/3"));
}

#[test]
fn each_preset_renders_a_different_reproducible_picture() {
    let verts = load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap()
    .expand_vertices(Vertex::DEFAULT_COLOR);
    let b = model_bounds(&verts).unwrap();
    let mut images = Vec::new();
    for preset in [
        ViewPreset::Oblique,
        ViewPreset::Top,
        ViewPreset::Front,
        ViewPreset::Side,
    ] {
        let view = OrbitView::new(
            OrbitCamera::new(b.center, preset),
            &b,
            Viewport::new(800, 600, 0.1),
        )
        .unwrap();
        let mut f1 = Framebuffer::new(800, 600);
        let mut f2 = Framebuffer::new(800, 600);
        render_interactive(
            &mut f1,
            &Uniforms::default(),
            &verts,
            view,
            TriangleMode::FilledScanline,
        )
        .unwrap();
        render_interactive(
            &mut f2,
            &Uniforms::default(),
            &verts,
            view,
            TriangleMode::FilledScanline,
        )
        .unwrap();
        assert_eq!(
            f1.to_rgba_bytes(),
            f2.to_rgba_bytes(),
            "{preset:?} is reproducible"
        );
        images.push(f1.to_rgba_bytes());
    }
    for i in 0..images.len() {
        for j in i + 1..images.len() {
            assert_ne!(images[i], images[j], "presets {i} and {j} look the same");
        }
    }
}

#[test]
fn reset_returns_to_the_starting_preset_and_t_switches_to_top() {
    let target = Vector3::new(0.0, 0.25, -1.3);
    let mut cam = OrbitCamera::new(target, ViewPreset::Side);
    let start = cam;
    apply_input(&mut cam, CameraAction::View(ViewPreset::Top), 0.0);
    assert_eq!((cam.azimuth, cam.elevation), ViewPreset::Top.angles());
    apply_input(&mut cam, CameraAction::OrbitLeft, 0.05);
    apply_input(&mut cam, CameraAction::ZoomIn, 0.05);
    apply_input(&mut cam, CameraAction::Reset, 0.0);
    assert_eq!(
        cam, start,
        "R restores the preset chosen on the command line"
    );
}

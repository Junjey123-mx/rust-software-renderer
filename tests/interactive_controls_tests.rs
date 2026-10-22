use raylib::math::Vector3;
use rust_software_renderer::app::{CameraAction, MAX_DT, ORBIT_SPEED, apply_input, parse_args};
use rust_software_renderer::pipeline::orbit_camera::{MAX_ELEVATION, MAX_ZOOM, MIN_ZOOM};
use rust_software_renderer::pipeline::{OrbitCamera, ViewPreset};
use rust_software_renderer::rasterization::TriangleMode;

fn cam() -> OrbitCamera {
    OrbitCamera::new(Vector3::new(0.0, 0.25, -1.3), ViewPreset::Oblique)
}

#[test]
fn held_actions_scale_with_frame_delta() {
    let mut c = cam();
    let a0 = c.azimuth;
    apply_input(&mut c, CameraAction::OrbitRight, 0.02);
    assert!((c.azimuth - a0 - ORBIT_SPEED * 0.02).abs() < 1e-6);
    apply_input(&mut c, CameraAction::OrbitLeft, 0.02);
    assert!((c.azimuth - a0).abs() < 1e-6, "left undoes right");
    let e0 = c.elevation;
    apply_input(&mut c, CameraAction::TiltUp, 0.01);
    assert!(c.elevation > e0);
    apply_input(&mut c, CameraAction::TiltDown, 0.01);
    assert!((c.elevation - e0).abs() < 1e-6);
}

#[test]
fn zoom_is_symmetric_and_clamped() {
    let mut c = cam();
    apply_input(&mut c, CameraAction::ZoomIn, 0.05);
    assert!(c.zoom > 1.0);
    apply_input(&mut c, CameraAction::ZoomOut, 0.05);
    assert!((c.zoom - 1.0).abs() < 1e-5);
    for _ in 0..10_000 {
        apply_input(&mut c, CameraAction::ZoomIn, MAX_DT);
    }
    assert_eq!(c.zoom, MAX_ZOOM);
    for _ in 0..10_000 {
        apply_input(&mut c, CameraAction::ZoomOut, MAX_DT);
    }
    assert_eq!(c.zoom, MIN_ZOOM);
}

#[test]
fn frame_delta_is_clamped_and_non_finite_is_ignored() {
    let mut a = cam();
    let mut b = cam();
    apply_input(&mut a, CameraAction::OrbitRight, 5.0);
    apply_input(&mut b, CameraAction::OrbitRight, MAX_DT);
    assert_eq!(a.azimuth, b.azimuth, "a 5 s stall moves as much as MAX_DT");
    let before = cam();
    let mut c = cam();
    apply_input(&mut c, CameraAction::TiltUp, f32::NAN);
    apply_input(&mut c, CameraAction::ZoomIn, f32::INFINITY);
    apply_input(&mut c, CameraAction::OrbitLeft, -1.0);
    assert_eq!(c, before);
}

#[test]
fn reset_and_presets_restore_known_orientations() {
    let mut c = cam();
    let initial = c;
    for _ in 0..30 {
        apply_input(&mut c, CameraAction::OrbitRight, MAX_DT);
        apply_input(&mut c, CameraAction::TiltUp, MAX_DT);
        apply_input(&mut c, CameraAction::ZoomIn, MAX_DT);
    }
    assert_ne!(c, initial);
    apply_input(&mut c, CameraAction::View(ViewPreset::Top), 0.016);
    assert_eq!(c.elevation, MAX_ELEVATION);
    assert_ne!(c.zoom, 1.0, "presets keep the zoom");
    apply_input(&mut c, CameraAction::Reset, 0.016);
    assert_eq!(c, initial);
    let (az, el) = ViewPreset::Side.angles();
    apply_input(&mut c, CameraAction::View(ViewPreset::Side), 0.0);
    assert_eq!((c.azimuth, c.elevation), (az, el));
}

#[test]
fn interactive_flag_is_parsed_and_compatible_with_other_flags() {
    let a = parse_args(
        [
            "--interactive",
            "--mode",
            "scanline",
            "--screenshot",
            "x.png",
        ]
        .map(String::from),
    )
    .unwrap();
    assert!(a.interactive);
    assert_eq!(a.mode, TriangleMode::FilledScanline);
    assert_eq!(a.screenshot.as_deref(), Some("x.png"));
    assert!(!parse_args(Vec::<String>::new()).unwrap().interactive);
    let b = parse_args(["--obj", "m.obj", "--interactive"].map(String::from)).unwrap();
    assert!(b.interactive && b.obj.is_some());
    assert!(
        parse_args(["--benchmark-triangles", "--profile"].map(String::from))
            .unwrap()
            .benchmark
    );
}

//! Evidence timelines and the headless RGB24 recorder.

use rust_software_renderer::app::parse_args;
use rust_software_renderer::celestial::cli::{extract, validate};
use rust_software_renderer::celestial::record::{RecordSettings, record};
use rust_software_renderer::celestial::timeline::{Timeline, TimelineState, state_at};
use rust_software_renderer::celestial::{Body, CelestialScene};
use rust_software_renderer::pipeline::{ShadedOptions, ViewPreset};
use rust_software_renderer::rasterization::TriangleMode;
use std::f32::consts::TAU;

fn base() -> TimelineState {
    TimelineState {
        azimuth: -2.45,
        elevation: 0.55,
        zoom: 1.0,
        light_azimuth: -1.3,
        light_elevation: 0.35,
        time: 8.0,
    }
}

#[test]
fn material_timeline_keeps_the_camera_fixed() {
    for i in 0..=24 {
        let t = i as f32;
        let s = state_at(Timeline::Material, base(), t, 24.0, false);
        assert_eq!((s.azimuth, s.elevation, s.zoom), (-2.45, 0.55, 1.0));
        assert_eq!((s.light_azimuth, s.light_elevation), (-1.3, 0.35));
        assert_eq!(s.time, 8.0 + t);
    }
}

#[test]
fn orbit_timeline_freezes_time_and_turns_once() {
    for i in 0..=12 {
        let s = state_at(Timeline::Orbit, base(), i as f32, 12.0, false);
        assert_eq!(s.time, 8.0, "orbit always freezes time");
        assert_eq!(s.light_azimuth, -1.3);
    }
    let end = state_at(Timeline::Orbit, base(), 12.0, 12.0, false);
    assert!((end.azimuth - (-2.45 + TAU)).abs() < 1e-5);
    let half = state_at(Timeline::Orbit, base(), 6.0, 12.0, false);
    assert!((half.azimuth - (-2.45 + TAU / 2.0)).abs() < 1e-5);
}

#[test]
fn light_timeline_keeps_camera_and_time_fixed() {
    for i in 0..=12 {
        let s = state_at(Timeline::Light, base(), i as f32, 12.0, true);
        assert_eq!(
            (s.azimuth, s.elevation, s.zoom, s.time),
            (-2.45, 0.55, 1.0, 8.0)
        );
        assert!((s.light_azimuth - (-1.3 + TAU * i as f32 / 12.0)).abs() < 1e-5);
    }
    let unfrozen = state_at(Timeline::Light, base(), 3.0, 12.0, false);
    assert_eq!(unfrozen.time, 11.0);
}

fn tiny_recording(seconds: f32, fps: u32) -> (usize, Vec<u8>) {
    let mut scene = CelestialScene::new(Body::HouseOfCards, ViewPreset::Oblique, 80, 60).unwrap();
    scene.time = 9.9;
    let settings = RecordSettings {
        mode: TriangleMode::FilledScanline,
        timeline: Timeline::Material,
        seconds,
        fps,
        freeze: false,
        options: ShadedOptions::default(),
        width: 80,
        height: 60,
    };
    let mut bytes = Vec::new();
    let frames = record(&scene, &settings, &mut bytes).unwrap();
    (frames, bytes)
}

#[test]
fn recording_writes_exact_frame_bytes() {
    let (frames, bytes) = tiny_recording(1.0, 2);
    assert_eq!(frames, 2);
    assert_eq!(bytes.len(), 2 * 80 * 60 * 3);
    let (frames, bytes) = tiny_recording(0.25, 10);
    assert_eq!(frames, 3, "ceil(seconds * fps)");
    assert_eq!(bytes.len(), 3 * 80 * 60 * 3);
}

#[test]
fn recording_is_deterministic() {
    let (_, a) = tiny_recording(1.0, 3);
    let (_, b) = tiny_recording(1.0, 3);
    assert_eq!(a, b);
    let frame = 80 * 60 * 3;
    assert_ne!(&a[..frame], &a[frame..2 * frame], "the material evolves");
}

#[test]
fn record_options_are_validated() {
    let check = |args: &[&str]| -> Result<(), String> {
        let (lab6, rest) = extract(args.iter().map(|s| s.to_string()).collect())?;
        validate(&parse_args(rest)?, &lab6)
    };
    assert!(
        check(&[
            "--scene",
            "house",
            "--record",
            "2",
            "--fps",
            "30",
            "--timeline",
            "orbit"
        ])
        .is_ok()
    );
    for bad in [
        &["--scene", "house", "--record", "0"][..],
        &["--scene", "house", "--record", "-1"],
        &["--scene", "house", "--record", "2", "--fps", "0"],
        &["--scene", "house", "--record", "2", "--fps", "200"],
        &["--scene", "house", "--record", "2", "--timeline", "spiral"],
        &["--scene", "house", "--record", "2", "--screenshot", "x.png"],
        &["--scene", "house", "--record", "2", "--interactive"],
        &["--scene", "house", "--fps", "30"],
        &["--scene", "ship", "--record", "2"],
    ] {
        assert!(check(bad).is_err(), "{bad:?} must be rejected");
    }
}

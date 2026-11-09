//! Headless recording of the solar scene.

use raylib::math::Vector3;
use rust_software_renderer::app::parse_args;
use rust_software_renderer::celestial::cli::{extract, validate};
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::solar::record::{
    FLIGHT_SECONDS, SolarRecordSettings, SolarTimeline, flight_actions, record_solar,
};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 80;
const H: u32 = 60;

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        5.3,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

fn settings(seconds: f32, fps: u32) -> SolarRecordSettings {
    SolarRecordSettings {
        timeline: SolarTimeline::Flight,
        seconds,
        fps,
        mode: TriangleMode::FilledScanline,
        options: ShadedOptions {
            threads: 2,
            ..ShadedOptions::default()
        },
        width: W,
        height: H,
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[test]
fn one_second_at_thirty_fps_is_exactly_thirty_rgb24_frames() {
    let (mut out, mut log) = (Vec::new(), Vec::new());
    let frames = record_solar(&scene(), &settings(1.0, 30), &mut out, &mut log).unwrap();
    assert_eq!(frames, 30);
    assert_eq!(
        out.len(),
        (W * H * 3) as usize * 30,
        "stdout holds only frames"
    );
    assert!(log.is_empty(), "no script event in the first second");
}

#[test]
fn two_recordings_are_identical_and_messages_go_to_the_log() {
    let (mut a, mut la) = (Vec::new(), Vec::new());
    let (mut b, mut lb) = (Vec::new(), Vec::new());
    record_solar(&scene(), &settings(4.0, 10), &mut a, &mut la).unwrap();
    record_solar(&scene(), &settings(4.0, 10), &mut b, &mut lb).unwrap();
    assert_eq!(fnv(&a), fnv(&b));
    assert_eq!(la, lb);
    let log = String::from_utf8(la).unwrap();
    assert!(log.contains("Warped to Crimson Entrance"), "{log}");
    assert!(log.contains("Flight mode: local"), "{log}");
    assert_eq!(a.len(), (W * H * 3) as usize * 40);
    // Frames change over time (the ship flies, the bodies orbit).
    let frame = (W * H * 3) as usize;
    assert_ne!(fnv(&a[..frame]), fnv(&a[39 * frame..]));
}

#[test]
fn the_flight_script_fires_every_event_exactly_once() {
    for fps in [10u32, 24, 30, 60] {
        let dt = 1.0 / fps as f32;
        let frames = (FLIGHT_SECONDS * fps as f32).ceil() as usize;
        let mut warps = 0;
        let mut toggles = 0;
        for f in 0..frames {
            let t = f as f32 * dt;
            let prev = if f == 0 { -1.0 } else { t - dt };
            for a in flight_actions(prev, t) {
                match a {
                    rust_software_renderer::solar::input::SolarAction::Warp(_) => warps += 1,
                    rust_software_renderer::solar::input::SolarAction::ToggleLocal => toggles += 1,
                    _ => {}
                }
            }
        }
        assert_eq!((warps, toggles), (2, 3), "fps {fps}");
    }
}

#[test]
fn the_cli_accepts_solar_recordings_and_keeps_timelines_apart() {
    let parse = |line: &str| {
        let (lab6, rest) = extract(line.split_whitespace().map(str::to_string).collect())?;
        let cli = parse_args(rest)?;
        validate(&cli, &lab6)?;
        Ok::<_, String>(lab6)
    };
    let l = parse("--scene solar --record 2 --fps 24 --timeline flight").unwrap();
    assert_eq!(
        (l.record, l.fps, l.solar_timeline),
        (Some(2.0), Some(24), Some(SolarTimeline::Flight))
    );
    assert!(parse("--scene solar --record 2").is_ok());
    assert!(parse("--scene solar --record 2 --timeline material").is_err());
    assert!(parse("--scene crown --record 2 --timeline flight").is_err());
    assert!(
        parse("--scene solar --timeline flight").is_err(),
        "needs --record"
    );
    assert!(
        parse("--scene crown --record 2 --timeline showcase").is_ok(),
        "lab 6 unchanged"
    );
    let err = parse("--scene solar --record 2 --timeline spin").unwrap_err();
    assert!(err.contains("flight") && err.contains("spin"), "{err}");
}

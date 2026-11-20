//! Project 3, close of P3-01 (C09-C15): high-level behavior after the move
//! to specs and attachments — orbits, pause, quick travel, rigs and local
//! flight — and the laboratory 6 sources left untouched. The frame
//! fingerprints live in `p3_frame_oracle_tests` and are not repeated here.
//!
//! `P3_PRINT_FROZEN=1 cargo test --test p3_refactor_regression_tests --
//! --nocapture` prints the current values of the frozen tables.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::flight::FlightMode;
use rust_software_renderer::solar::input::SolarAction;
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};
use std::path::Path;

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

fn printing() -> bool {
    std::env::var_os("P3_PRINT_FROZEN").is_some()
}

fn near(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-5
}

#[test]
fn orbits_unchanged() {
    // The pre-project orbits (radius, rad/s, phase) and spins at three
    // instants, from the formulas of `ea3b4a8`.
    let wired = [
        (0.0, 0.0, 0.0, 0.05),
        (3.0, 0.5, 0.0, 0.6),
        (6.2, 0.22, std::f32::consts::PI, 0.35),
    ];
    let mut s = scene();
    for t in [0.0f32, 3.7, 60.0] {
        s.clock.time = t;
        s.update();
        for (body, &(r, w, phase, spin)) in s.bodies.iter().zip(&wired) {
            let a = phase + w * t;
            let expected = Vector3::new(r * a.cos(), 0.0, r * a.sin());
            assert!(
                near(body.transform.position, expected),
                "t {t}: {:?}",
                body.body()
            );
            assert_eq!(body.transform.yaw, spin * t);
        }
    }
}

#[test]
fn pause_semantics_unchanged() {
    let mut s = scene();
    s.clock.time = 2.0;
    s.update();
    let messages = apply_toggles(&mut s, &[SolarAction::TogglePause]);
    assert!(s.clock.paused && messages[0].starts_with("Time paused"));
    let ship = s.ship.position;
    let planet = s.bodies[1].transform.position;
    // Paused: the simulated time and the orbits hold, the real delta still
    // runs and the ship still flies.
    step(
        &mut s,
        &[SolarAction::Forward],
        &[],
        Default::default(),
        0.1,
    );
    assert_eq!(s.clock.time, 2.0);
    assert_eq!(s.clock.delta, 0.0);
    assert!(s.clock.real_delta > 0.0);
    assert_eq!(s.bodies[1].transform.position, planet);
    assert!((s.ship.position - ship).length() > 0.0);
    apply_toggles(&mut s, &[SolarAction::TogglePause]);
    step(&mut s, &[], &[], Default::default(), 0.1);
    assert!(s.clock.time > 2.0);
}

/// Arrival of `warp_to` next to each body at t = 6 s (frozen after `C15`;
/// `travel.rs` is unchanged since `ea3b4a8`).
const WARP_ARRIVALS: [[f32; 3]; 3] = [
    [0.0, 0.15470992, 2.3206487],
    [-2.4085104, 0.1134285, 2.0447526],
    [-1.3369422, 0.07866916, -4.0385137],
];

#[test]
fn warp_unchanged() {
    let mut got = Vec::new();
    for i in 0..3 {
        let mut s = scene();
        s.clock.time = 6.0;
        s.update();
        s.flight = FlightMode::Local {
            planet: 1,
            previous_planet_position: s.bodies[1].transform.position,
        };
        let messages = apply_toggles(&mut s, &[SolarAction::Warp(i)]);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert_eq!(s.flight, FlightMode::Space, "warp leaves local flight");
        let to_body = (s.bodies[i].transform.position - s.ship.position).normalized();
        assert!(to_body.dot(s.ship.forward) > 0.999, "nose towards the body");
        let p = s.ship.position;
        got.push([p.x, p.y, p.z]);
    }
    if printing() {
        println!("WARP_ARRIVALS = {got:?}");
    }
    for (g, w) in got.iter().zip(WARP_ARRIVALS) {
        assert!(
            near(
                Vector3::new(g[0], g[1], g[2]),
                Vector3::new(w[0], w[1], w[2])
            ),
            "{got:?}"
        );
    }
}

#[test]
fn rigs_and_local_flight_unchanged() {
    let mut s = scene();
    assert_eq!(s.rig, CameraRig::Follow);
    apply_toggles(&mut s, &[SolarAction::ToggleCamera]);
    assert_eq!(s.rig, CameraRig::Orbit);
    apply_toggles(&mut s, &[SolarAction::ToggleCamera]);
    assert_eq!(s.rig, CameraRig::Follow);
    // L next to Crimson Entrance enters local flight; L again leaves it.
    let p = s.bodies[1].transform.position;
    s.ship.position = p + Vector3::new(0.0, s.radius_of(1) + 1.0, 0.0);
    apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
    assert!(matches!(s.flight, FlightMode::Local { planet: 1, .. }));
    apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
    assert_eq!(s.flight, FlightMode::Space);
    // Far from every planet it is refused.
    s.ship.position = Vector3::new(0.0, 40.0, 0.0);
    let messages = apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
    assert!(messages[0].starts_with("Local flight unavailable"));
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Laboratory 6 sources (all of `src/celestial/` but `cli.rs`, which takes
/// the additive solar flags), hashed with `\r\n` read as `\n`. Equal to
/// `ea3b4a8`: `git diff ea3b4a8 -- src/celestial` only lists `cli.rs`.
const CELESTIAL_SOURCES: [(&str, u64); 11] = [
    ("src/celestial/crown.rs", 0xccda02e3b2a53cf2),
    ("src/celestial/entrance/clouds.rs", 0x74c42be65d755d4d),
    ("src/celestial/entrance/events.rs", 0x8f23f206dd4e6f21),
    ("src/celestial/entrance/mod.rs", 0x92543df0ca151bec),
    ("src/celestial/entrance/terrain.rs", 0x993bff5d8c7ce972),
    ("src/celestial/house.rs", 0x4e317d15e74a062a),
    ("src/celestial/mod.rs", 0xc2b5c3eca9588e27),
    ("src/celestial/record.rs", 0xdfababf6a183dad6),
    ("src/celestial/scene.rs", 0x7cd4eeb4c0d7ebb9),
    ("src/celestial/timeline.rs", 0x590fde161a9e9222),
    ("src/celestial/viewer.rs", 0xacebb7bcdefc28d7),
];

fn source_hash(path: &str) -> u64 {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
    fnv1a64(text.replace("\r\n", "\n").as_bytes())
}

#[test]
fn celestial_sources_untouched() {
    let got: Vec<(&str, u64)> = CELESTIAL_SOURCES
        .iter()
        .map(|&(p, _)| (p, source_hash(p)))
        .collect();
    if printing() {
        for (p, h) in &got {
            println!("    (\"{p}\", {h:#018x}),");
        }
    }
    assert_eq!(got, CELESTIAL_SOURCES);
    // Every source file of the directory is listed (cli.rs aside).
    let mut files = Vec::new();
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src/celestial")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.file_name().unwrap() != "cli.rs" {
                files.push(path);
            }
        }
    }
    assert_eq!(files.len(), CELESTIAL_SOURCES.len());
}

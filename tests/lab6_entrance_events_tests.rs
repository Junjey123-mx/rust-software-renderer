//! Crimson Entrance: migrating craters and spade mesas.

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::entrance::events::{
    CRATER_CONE_DEG, CRATER_SLOTS, EventKind, SPADE_CONE_DEG, active_events, crater_relief,
    spade_relief,
};
use rust_software_renderer::celestial::entrance::terrain;
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::pipeline::{ShaderUniforms, direction_from_angles};
use rust_software_renderer::shading::sphere::{angular_distance, from_local};

fn directions(n: usize) -> Vec<Vector3> {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).sqrt();
            let phi = golden * i as f32;
            Vector3::new(r * phi.cos(), y, r * phi.sin())
        })
        .collect()
}

fn frame_at(params: EntranceParams, time: f32) -> Frame {
    let u = ShaderUniforms {
        time,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    };
    prepare(&params, &u, DebugView::None)
}

fn p() -> EntranceParams {
    EntranceParams::default()
}

#[test]
fn events_vanish_at_cycle_boundaries() {
    let dirs = directions(4096);
    for boundary in [p().crater_period, p().spade_period, 2.0 * p().crater_period] {
        let before = frame_at(p(), boundary - 1e-3);
        let after = frame_at(p(), boundary + 1e-3);
        let worst = dirs
            .iter()
            .map(|&n| (terrain::macro_height(n, &before) - terrain::macro_height(n, &after)).abs())
            .fold(0.0, f32::max);
        assert!(worst < 1e-2, "boundary {boundary}: jump {worst}");
    }
}

#[test]
fn events_relocate_every_cycle() {
    let period = p().crater_period;
    let centers: Vec<Vector3> = (0..10)
        .map(|k| frame_at(p(), (k as f32 + 0.5) * period).events[0].center)
        .collect();
    let steps: Vec<f32> = centers
        .windows(2)
        .map(|w| angular_distance(w[0], w[1]))
        .collect();
    assert!(steps.iter().all(|&d| d > 0.05), "{steps:?}");
    let mean = steps.iter().sum::<f32>() / steps.len() as f32;
    assert!(mean > 0.4, "mean relocation {mean}");
}

#[test]
fn spade_mesa_rises_at_apogee() {
    let f = frame_at(p(), 8.0);
    let e = f.events[CRATER_SLOTS];
    assert_eq!(e.kind, EventKind::Spade);
    assert!((e.tau - 0.5).abs() < 1e-4, "spade 0 at apogee at t = 8");
    let top = spade_relief(e.center, &e, f.seed).height;
    assert!(top > 0.8 * p().spade_height, "{top}");
    let away = from_local(e.center, e.east, e.north, 1.3 * e.radius, 0.0);
    assert!(spade_relief(away, &e, f.seed).height < 0.1);
}

#[test]
fn spade_relief_is_mirror_symmetric_at_apogee() {
    let f = frame_at(p(), 8.0);
    let e = f.events[CRATER_SLOTS];
    for i in 0..30 {
        for j in 0..30 {
            let (u, v) = (-0.3 + i as f32 * 0.02, -0.3 + j as f32 * 0.02);
            let a = spade_relief(from_local(e.center, e.east, e.north, u, v), &e, f.seed);
            let b = spade_relief(from_local(e.center, e.east, e.north, -u, v), &e, f.seed);
            assert!((a.height - b.height).abs() < 1e-3, "({u}, {v})");
        }
    }
}

#[test]
fn crater_floor_sinks_during_the_sink_phase() {
    let period = p().crater_period;
    let depth_at = |tau: f32| {
        let f = frame_at(p(), tau * period);
        let e = f.events[0];
        crater_relief(e.center, &e).height
    };
    let (opening, sinking, mature) = (depth_at(0.1), depth_at(0.3), depth_at(0.5));
    assert!(opening.abs() < 1e-3, "{opening}");
    assert!(
        sinking < opening && mature < sinking,
        "{opening} {sinking} {mature}"
    );
    assert!(mature < -0.8 * p().crater_depth);
}

#[test]
fn events_stay_in_the_visible_cone() {
    for i in 0..200 {
        let f = frame_at(p(), i as f32 * 0.73);
        for e in f.events.iter() {
            let cone = match e.kind {
                EventKind::Crater => CRATER_CONE_DEG,
                EventKind::Spade => SPADE_CONE_DEG,
            };
            assert!(angular_distance(e.center, f.focus) <= cone.to_radians() + 1e-4);
        }
    }
    let events = active_events(&p(), 11, frame_at(p(), 0.0).focus, 3.0);
    assert_eq!(events.len(), 5);
}

#[test]
fn events_switch_disables_all_events() {
    let off = EntranceParams { events: 0.0, ..p() };
    let dirs = directions(2048);
    let (a, b) = (frame_at(off, 0.0), frame_at(off, 7.5));
    for &n in &dirs {
        assert_eq!(terrain::height(n, &a), terrain::height(n, &b));
    }
    let (on, at_rest) = (frame_at(p(), 8.0), frame_at(off, 8.0));
    let changed = dirs
        .iter()
        .filter(|&&n| (terrain::height(n, &on) - terrain::height(n, &at_rest)).abs() > 1e-3)
        .count();
    assert!(changed > 20, "events change the relief ({changed})");
}

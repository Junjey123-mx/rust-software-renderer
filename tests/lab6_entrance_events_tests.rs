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

// ---- Artistic complement: the spade burned into the crust ----

use rust_software_renderer::celestial::entrance::events::spade_brand;
use rust_software_renderer::pipeline::SurfaceSample;
use rust_software_renderer::shading::sdf::spade;

/// Directions of spade 0 at plane points given in spade units.
fn spade_point(f: &Frame, x: f32, y: f32) -> Vector3 {
    let e = f.events[CRATER_SLOTS];
    from_local(e.center, e.east, e.north, x * e.radius, y * e.radius)
}

#[test]
fn branded_spade_fills_its_interior_at_apogee() {
    let f = frame_at(p(), 8.0);
    let e = f.events[CRATER_SLOTS];
    let (mut inside, mut lit) = (0, 0);
    for i in 0..40 {
        for j in 0..40 {
            let (x, y) = (-0.7 + i as f32 * 0.035, -0.7 + j as f32 * 0.035);
            let s = spade((x, y));
            let relief = spade_relief(spade_point(&f, x, y), &e, f.seed);
            if s < -0.06 {
                inside += 1;
                if relief.brand > 0.5 {
                    lit += 1;
                }
            } else if s > 0.25 {
                assert!(relief.brand < 1e-6, "outside the symbol at ({x}, {y})");
            }
        }
    }
    assert!(
        inside > 100 && lit * 10 >= inside * 9,
        "{lit} of {inside} interior points burn"
    );
    let center = spade_relief(spade_point(&f, 0.0, 0.1), &e, f.seed);
    assert!(center.brand_core > 0.8, "white-hot core at the apogee");
}

#[test]
fn charred_rim_surrounds_the_burning_spade() {
    let f = frame_at(p(), 8.0);
    let n = spade_point(&f, 0.0, 0.0);
    for (s, expect_char) in [(-0.2, false), (0.08, true), (0.4, false)] {
        let (_, _, char) = spade_brand(n, s, 0.5, f.seed);
        assert_eq!(char > 0.3, expect_char, "s = {s}: char {char}");
    }
}

#[test]
fn spade_brand_burns_and_cools_cyclically() {
    let n = Vector3::new(0.3, 0.4, 0.86).normalized();
    let fill = |tau: f32| spade_brand(n, -0.2, tau, 11).0;
    let outline = |tau: f32| spade_brand(n, 0.0, tau, 11).0;
    assert_eq!(fill(0.0), 0.0);
    assert_eq!(fill(0.04), 0.0, "subterranean heating only");
    assert!(
        outline(0.16) > 0.3 && fill(0.16) < 0.2,
        "the outline is engraved first"
    );
    assert!(fill(0.5) > 0.5, "the interior burns at the apogee");
    assert_eq!(fill(0.97), 0.0, "the symbol is gone before the next cycle");
    for tau in [0.999, 1.0 - 1e-5] {
        let (b, c, ch) = spade_brand(n, 0.05, tau, 11);
        assert!(
            b < 1e-3 && c < 1e-3 && ch < 1e-3,
            "continuous at the cycle end"
        );
    }
}

#[test]
fn branded_spade_reads_hyperintense_red_by_day_and_night() {
    let f = frame_at(EntranceParams { clouds: 0.0, ..p() }, 5.3);
    let red = f.brand_emission(1.0, 0.0);
    assert!(red.x >= 1.4 && red.y < 0.3 && red.z < 0.3, "{red:?}");
    // The brand is self-lit: shade a burning point under opposite lights.
    let n = spade_point(&f, 0.0, 0.1);
    let sample = SurfaceSample {
        x: 0,
        y: 0,
        depth: 0.0,
        object_position: n,
        normal: n,
        face_normal: n,
    };
    for light in [n, -n] {
        let u = ShaderUniforms {
            time: 5.3,
            light_direction: light,
            eye_direction: direction_from_angles(-2.45, 0.55),
        };
        let lit = prepare(&EntranceParams { clouds: 0.0, ..p() }, &u, DebugView::None);
        let c = lit.shade(&sample);
        assert!(c.x >= 0.95 && c.x > 1.6 * c.y && c.x > 3.0 * c.z, "{c:?}");
    }
}

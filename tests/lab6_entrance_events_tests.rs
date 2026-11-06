//! Crimson Entrance: spade craters, the planet's single geological event.

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::entrance::events::{
    EVENT_SLOTS, INFLUENCE, LIFETIME, SLOT_SIZE, SpadeCrater, profile, slot_cycle, stages,
};
use rust_software_renderer::celestial::entrance::terrain;
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::pipeline::{ShaderUniforms, SurfaceSample, direction_from_angles};
use rust_software_renderer::shading::sdf::spade;
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

/// Time at which slot `slot` is at phase `tau` of its first cycle.
fn time_of(slot: usize, tau: f32) -> f32 {
    let period = p().event_period;
    slot as f32 * period / 3.0 + tau * period
}

/// Direction of plane point `(x, y)` in spade units of event `e`.
fn at(e: &SpadeCrater, x: f32, y: f32) -> Vector3 {
    from_local(e.center, e.east, e.north, x * e.size, y * e.size)
}

/// Spade-unit grid points of event `e` with their exact signed distance.
fn grid(e: &SpadeCrater) -> Vec<(Vector3, f32)> {
    let mut points = Vec::new();
    for i in 0..48 {
        for j in 0..48 {
            let (x, y) = (-1.0 + i as f32 / 23.5, -1.0 + j as f32 / 23.5);
            points.push((at(e, x, y), spade((x, y))));
        }
    }
    points
}

#[test]
fn craters_and_spades_are_one_system_of_three_slots() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/celestial/entrance/events.rs"),
    )
    .unwrap();
    for gone in ["EventKind", "crater_relief", "spade_relief", "SPADE_SLOTS"] {
        assert!(!source.contains(gone), "independent logic left: {gone}");
    }
    assert_eq!(EVENT_SLOTS, 3);
    for i in 0..300 {
        let f = frame_at(p(), i as f32 * 0.37);
        assert_eq!(f.events.len(), 3);
        let alive = f.events.iter().filter(|e| e.alive()).count();
        assert!(alive <= 2, "at most two spade craters are open at once");
        for (slot, e) in f.events.iter().enumerate() {
            assert_eq!(e.slot, slot);
            assert_eq!(e.index.rem_euclid(3) as usize, slot);
        }
    }
}

#[test]
fn slots_are_large_medium_and_small() {
    let f = frame_at(p(), 0.0);
    let e = &f.events;
    assert!(e[0].size > e[1].size && e[1].size > e[2].size);
    assert!(e[0].depth > e[1].depth && e[1].depth > e[2].depth);
    assert!(e[0].glow_peak > e[2].glow_peak);
    assert_eq!(e[0].size, p().event_size * SLOT_SIZE[0]);
}

#[test]
fn slots_run_a_third_of_a_period_apart() {
    let period = p().event_period;
    for i in 0..200 {
        let t = i as f32 * 0.61;
        let taus: Vec<f32> = (0..3).map(|s| slot_cycle(period, s, t).1).collect();
        for s in 0..3 {
            let gap = (taus[s] - taus[(s + 1) % 3]).rem_euclid(1.0);
            assert!((gap - 1.0 / 3.0).abs() < 1e-3, "t {t}: {taus:?}");
        }
    }
    // Events are born in order: large, medium, small, large...
    assert_eq!(slot_cycle(period, 0, 0.5).0, 0);
    assert_eq!(slot_cycle(period, 1, period / 3.0 + 0.5).0, 1);
    assert_eq!(slot_cycle(period, 2, 2.0 * period / 3.0 + 0.5).0, 2);
    assert_eq!(slot_cycle(period, 0, period + 0.5).0, 3);
}

#[test]
fn stages_start_and_end_at_zero() {
    let zero = stages(0.0);
    assert_eq!(zero, Default::default());
    for tau in [LIFETIME, 0.7, 0.9, 0.999] {
        assert_eq!(stages(tau), Default::default(), "quiet at {tau}");
    }
    let edge = |tau: f32| {
        let s = stages(tau);
        [s.fracture, s.depth, s.glow, s.cooling, s.scorch]
            .into_iter()
            .fold(0.0, f32::max)
    };
    assert!(edge(1e-4) < 1e-4 && edge(LIFETIME - 1e-4) < 1e-4);
}

#[test]
fn incandescence_peaks_after_the_full_depth() {
    let samples: Vec<(f32, f32, f32)> = (0..=600)
        .map(|i| {
            let tau = i as f32 / 1000.0;
            let s = stages(tau);
            (tau, s.depth, s.glow)
        })
        .collect();
    let full_depth = samples.iter().find(|s| s.1 >= 0.999).unwrap().0;
    let first_glow = samples.iter().find(|s| s.2 > 0.0).unwrap().0;
    let peak = samples.iter().find(|s| s.2 >= 0.999).unwrap().0;
    assert!(first_glow > full_depth, "{first_glow} vs {full_depth}");
    assert!(peak > full_depth + 0.1);
    // Fractures announce the ♠ while the ground has barely moved.
    let early = stages(0.06);
    assert!(early.fracture > 0.5 && early.depth < 0.05);
}

#[test]
fn the_basin_is_carved_by_the_spade_distance() {
    assert_eq!(profile(-0.3), -1.0);
    assert!(profile(INFLUENCE).abs() < 1e-6);
    let f = frame_at(p(), time_of(0, 0.3));
    let e = f.events[0];
    let amount = e.amount();
    assert!(amount > 0.99 * e.depth, "full depth at tau 0.3");
    for (n, s) in grid(&e) {
        let h = e.height(n);
        if s < -0.15 {
            assert!(h < -0.7 * amount, "floor at s {s}: {h}");
        } else if s > INFLUENCE + 0.05 {
            assert!(h.abs() < 1e-6, "untouched ground at s {s}: {h}");
        }
    }
    // Two points at the same distance from the center: inside the blade
    // the ground is deep, beside it (outside the ♠) it is not. A circular
    // crater could not tell them apart.
    assert!(e.height(at(&e, 0.0, 0.3)) < -0.7 * amount);
    assert!(e.height(at(&e, 0.0, 0.48)) < -0.3 * amount);
    assert!(e.height(at(&e, 0.34, 0.34)) > -0.05 * amount);
}

#[test]
fn surface_and_height_lookups_agree() {
    for t in [2.0, 6.3, 8.0, 10.5] {
        let f = frame_at(p(), t);
        for e in f.events.iter() {
            for (n, _) in grid(e) {
                assert_eq!(e.surface(n).height, e.height(n));
            }
        }
    }
}

#[test]
fn terrain_returns_to_its_base_height_when_a_crater_closes() {
    let off = EntranceParams { events: 0.0, ..p() };
    for tau in [LIFETIME, 0.75, 0.99] {
        let t = time_of(0, tau);
        let (on, base) = (frame_at(p(), t), frame_at(off, t));
        let e = on.events[0];
        assert!(!e.alive());
        for (n, _) in grid(&e) {
            assert_eq!(e.surface(n), Default::default());
            let others: f32 = on.events[1..].iter().map(|o| o.height(n)).sum();
            let back = terrain::macro_height(n, &on) - others;
            assert!((back - terrain::macro_height(n, &base)).abs() < 1e-6);
        }
    }
}

#[test]
fn events_vanish_at_cycle_boundaries() {
    let dirs = directions(4096);
    let third = p().event_period / 3.0;
    for k in 1..=6 {
        let boundary = k as f32 * third;
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
fn a_crater_stays_put_while_visible_and_moves_between_cycles() {
    let period = p().event_period;
    for slot in 0..3 {
        let born = frame_at(p(), time_of(slot, 0.001)).events[slot];
        for i in 1..=60 {
            let e = frame_at(p(), time_of(slot, LIFETIME * i as f32 / 60.0 - 1e-3)).events[slot];
            assert_eq!(e.index, born.index);
            assert_eq!(e.center, born.center);
            assert_eq!((e.east, e.north), (born.east, born.north));
        }
    }
    let centers: Vec<Vector3> = (0..10)
        .map(|k| frame_at(p(), (k as f32 + 0.3) * period).events[0].center)
        .collect();
    let steps: Vec<f32> = centers
        .windows(2)
        .map(|w| angular_distance(w[0], w[1]))
        .collect();
    assert!(steps.iter().all(|&d| d > 0.05), "{steps:?}");
}

#[test]
fn same_time_and_seed_give_the_same_events() {
    for t in [0.0, 5.0, 33.3, 120.0] {
        let (a, b) = (frame_at(p(), t), frame_at(p(), t));
        assert_eq!(a.events, b.events);
        let other = frame_at(
            EntranceParams {
                seed: 4242.0,
                ..p()
            },
            t,
        );
        assert_ne!(a.events[0].center, other.events[0].center);
    }
}

#[test]
fn the_molten_floor_fills_the_spade_at_its_peak_and_nowhere_else() {
    let f = frame_at(p(), time_of(0, 0.38));
    let e = f.events[0];
    let (mut inside, mut lit, mut white) = (0, 0, 0.0_f32);
    for (n, s) in grid(&e) {
        let surface = e.surface(n);
        if s < -0.1 {
            inside += 1;
            if surface.lava > 0.4 {
                lit += 1;
            }
            white = white.max(surface.core);
        } else if s > 0.06 {
            assert_eq!(surface.lava, 0.0, "lava outside the ♠ at s {s}");
        }
    }
    assert!(inside > 100 && lit * 10 >= inside * 9, "{lit} of {inside}");
    assert!(white > 0.5, "white-hot core at the peak: {white}");
}

#[test]
fn fractures_then_scorch_then_cooled_crust() {
    let mask = |tau: f32, x: f32, y: f32| {
        let f = frame_at(p(), time_of(0, tau));
        let e = f.events[0];
        e.surface(at(&e, x, y))
    };
    // The outline cracks first, before any glow.
    let outline_x = (0..300)
        .map(|i| i as f32 * 0.002)
        .find(|&x| spade((x, -0.08)) > -0.005)
        .unwrap();
    let early = mask(0.07, outline_x, -0.08);
    assert!(early.fracture > 0.4 && early.lava == 0.0, "{early:?}");
    // A carbonized band surrounds the open crater.
    assert!(mask(0.3, outline_x + 0.06, -0.08).char > 0.3);
    // While it cools, crust replaces part of the floor.
    let floor = |tau: f32| {
        let f = frame_at(p(), time_of(0, tau));
        let e = f.events[0];
        grid(&e)
            .into_iter()
            .filter(|&(_, s)| s < -0.1)
            .map(|(n, _)| e.surface(n))
            .fold((0.0, 0.0), |(l, c), s| (l + s.lava, c + s.char))
    };
    let (peak_lava, _) = floor(0.38);
    let (cool_lava, cool_char) = floor(0.48);
    assert!(cool_lava < 0.6 * peak_lava && cool_char > 0.0);
    let (gone, _) = floor(LIFETIME);
    assert_eq!(gone, 0.0, "no emission once the cycle ends");
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

#[test]
fn molten_spade_reads_hyperintense_red_by_day_and_night() {
    let t = time_of(0, 0.38);
    let f = frame_at(EntranceParams { clouds: 0.0, ..p() }, t);
    let red = f.lava_emission(1.0, 0.0);
    assert!(red.x >= 1.4 && red.y < 0.4 && red.z < 0.4, "{red:?}");
    // The lava is self-lit: shade a molten point under opposite lights.
    let e = f.events[0];
    let n = at(&e, 0.0, 0.1);
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
            time: t,
            light_direction: light,
            eye_direction: n,
        };
        let lit = prepare(&EntranceParams { clouds: 0.0, ..p() }, &u, DebugView::None);
        let c = lit.shade(&sample);
        assert!(c.x >= 0.95 && c.x > 1.6 * c.y && c.x > 3.0 * c.z, "{c:?}");
    }
}

// ---- Spatial and temporal distribution ----

use rust_software_renderer::celestial::entrance::events::{
    FRONT_CONE_DEG, HISTORY, ORIGIN, clearance, place_events,
};

#[test]
fn open_spade_craters_never_overlap() {
    let margin = p().event_margin;
    for i in 0..2400 {
        let f = frame_at(p(), i as f32 * 0.25);
        let alive: Vec<&SpadeCrater> = f.events.iter().filter(|e| e.alive()).collect();
        for (a, ea) in alive.iter().enumerate() {
            for eb in &alive[a + 1..] {
                let gap = angular_distance(ea.center, eb.center);
                let needed = ea.reach_cos.acos() + eb.reach_cos.acos() + margin;
                assert!(
                    gap >= needed - 1e-4,
                    "t {}: {gap} < {needed}",
                    i as f32 * 0.25
                );
            }
        }
    }
}

#[test]
fn new_craters_keep_clear_of_recently_closed_zones() {
    let f = frame_at(p(), 0.0);
    let placed = place_events(&p(), f.seed, f.focus, 400);
    assert_eq!(placed[0].index, ORIGIN);
    for i in 1..placed.len() {
        let recent = &placed[i.saturating_sub(HISTORY)..i];
        let gap = clearance(placed[i].center, placed[i].reach, recent, p().event_margin);
        assert!(gap >= 0.0, "event {}: clearance {gap}", placed[i].index);
    }
}

#[test]
fn placement_depends_only_on_the_event_index() {
    let f = frame_at(p(), 0.0);
    let short = place_events(&p(), f.seed, f.focus, 40);
    let long = place_events(&p(), f.seed, f.focus, 90);
    assert_eq!(short[..], long[..short.len()]);
    // The frame uses exactly these centers.
    let third = p().event_period / 3.0;
    for m in 0..40_i64 {
        let frame = frame_at(p(), m as f32 * third + 0.5);
        let e = frame.events[m.rem_euclid(3) as usize];
        assert_eq!(e.index, m);
        assert_eq!(e.center, long[(m - ORIGIN) as usize].center);
    }
}

#[test]
fn craters_spread_over_the_planet_and_open_in_view() {
    let f = frame_at(p(), 0.0);
    let placed = place_events(&p(), f.seed, f.focus, 200);
    let front = (FRONT_CONE_DEG.to_radians() + 1e-3).cos();
    let even: Vec<_> = placed.iter().filter(|e| e.index % 2 == 0).collect();
    let in_view = even
        .iter()
        .filter(|e| e.center.dot(f.focus) >= front)
        .count();
    assert!(
        in_view * 10 >= even.len() * 8,
        "{in_view} of {}",
        even.len()
    );
    let far_side = placed
        .iter()
        .filter(|e| e.center.dot(f.focus) < 0.0)
        .count();
    assert!(far_side * 6 >= placed.len(), "{far_side} on the far side");
    // The opening scene shows the large crater forming in view.
    let first = f.events[0];
    assert_eq!(first.index, 0);
    assert!(first.center.dot(f.focus) >= front);
}

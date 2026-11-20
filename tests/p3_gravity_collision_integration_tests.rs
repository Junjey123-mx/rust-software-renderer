//! Project 3 gravity and collision, end to end on the Project 3 system
//! (C74): what the teacher sees. With `G` the coasting ship curves towards
//! a planet and without it goes straight; the pull of two planets adds up
//! and stays finite at a center; boosting into any of the four planets,
//! after a quick travel or in local flight never goes through the ground;
//! the path is the same at 30, 60 and 144 FPS and the window shows the
//! gravity state.
//!
//! `P3_DUMP_DIR=<dir>` writes the straight and curved paths as
//! `gravity_trajectories.csv` (`docs/results/project3/gravity_evidence.md`).

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::FlightMode;
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::physics::{GravitySource, gravity_accel};
use rust_software_renderer::solar::travel::warp_to;
use rust_software_renderer::solar::viewer::{apply_toggles, status_line, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const DT: f32 = 1.0 / 60.0;
const ENTRANCE: usize = 1;

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

/// Paused scene (static field), the ship coasting past Crimson Entrance at
/// three radii, 1 u/s across, dampers off.
fn coasting(gravity: bool) -> SolarScene {
    let mut s = scene();
    apply_toggles(&mut s, &[SolarAction::TogglePause]);
    s.gravity = gravity;
    let planet = s.bodies[ENTRANCE].transform.position;
    s.ship.position = planet + Vector3::new(0.0, 3.0 * s.radius_of(ENTRANCE), 0.0);
    s.ship.velocity = Vector3::new(1.0, 0.0, 0.0);
    s.ship.dampers = false;
    s
}

/// Positions every `dt` over `seconds` of coasting.
fn coast(s: &mut SolarScene, seconds: f32, dt: f32) -> Vec<Vector3> {
    (0..(seconds / dt).round() as usize)
        .map(|_| {
            step(s, &[], &[], Look::default(), dt);
            s.ship.position
        })
        .collect()
}

/// Gap between the ship's sphere and the highest ground of body `i`.
fn gap(s: &SolarScene, i: usize) -> f32 {
    let relief = s.relief_of(i).map_or(0.0, |r| r.field.max_height());
    (s.ship.position - s.bodies[i].transform.position).length()
        - s.radius_of(i) * (1.0 + relief)
        - s.ship.radius()
}

/// Distance of `p` from the line through `a` along `dir`.
fn off_line(p: Vector3, a: Vector3, dir: Vector3) -> f32 {
    let d = p - a;
    (d - dir * d.dot(dir)).length()
}

#[test]
fn straight_vs_curved() {
    let (mut off, mut on) = (coasting(false), coasting(true));
    let start = off.ship.position;
    let planet = on.bodies[ENTRANCE].transform.position;
    let r = on.radius_of(ENTRANCE);
    let straight = coast(&mut off, 6.0, DT);
    let curved = coast(&mut on, 6.0, DT);
    let x = Vector3::new(1.0, 0.0, 0.0);
    // Off: on the line. On: bent towards the planet by more than half a
    // radius, and never into it.
    assert!(straight.iter().all(|&p| off_line(p, start, x) < 1e-4));
    let drop = start.y - curved.last().unwrap().y;
    assert!(drop > 0.5 * r, "{drop} of R {r}");
    assert!(curved.iter().all(|&p| (p - planet).length() > r));
    // Negative case: gravity on but no mass anywhere is exactly straight.
    let mut massless = coasting(true);
    for b in &mut massless.bodies {
        b.mu = 0.0;
    }
    assert!(massless.gravity_sources().is_empty());
    let flat = coast(&mut massless, 6.0, DT);
    for (a, b) in flat.iter().zip(&straight) {
        assert!((*a - *b).length() < 1e-4);
    }
    if let Some(dir) = std::env::var_os("P3_DUMP_DIR") {
        let mut csv = String::from("t,off_x,off_y,off_z,on_x,on_y,on_z\n");
        for (k, (a, b)) in straight.iter().zip(&curved).enumerate().step_by(15) {
            let (a, b) = (*a - planet, *b - planet);
            csv += &format!(
                "{:.2},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\n",
                (k + 1) as f32 * DT,
                a.x,
                a.y,
                a.z,
                b.x,
                b.y,
                b.z
            );
        }
        std::fs::write(
            std::path::Path::new(&dir).join("gravity_trajectories.csv"),
            csv,
        )
        .unwrap();
    }
}

#[test]
fn two_planet_superposition() {
    let s = scene();
    let sources = s.gravity_sources();
    assert_eq!(sources.len(), 4, "the four planets, not the star");
    // Between Crimson Entrance and the clover the pulls add up.
    let (a, b) = (sources[0], sources[1]);
    let p = (a.center + b.center) * 0.5 + Vector3::new(0.0, 2.0, 0.0);
    let both = gravity_accel(p, &[a, b]);
    let sum = gravity_accel(p, &[a]) + gravity_accel(p, &[b]);
    assert!((both - sum).length() < 1e-6 * sum.length().max(1.0));
    // Each pulls towards its own center.
    assert!(gravity_accel(p, &[a]).dot(a.center - p) > 0.0);
    assert!(gravity_accel(p, &[b]).dot(b.center - p) > 0.0);
    // The whole system is the sum of all four.
    let all: Vector3 = sources
        .iter()
        .fold(Vector3::zero(), |acc, s| acc + gravity_accel(p, &[*s]));
    assert!((gravity_accel(p, &sources) - all).length() < 1e-6);
}

#[test]
fn center_softening() {
    let s = scene();
    let source = s.gravity_sources()[0];
    let GravitySource {
        center,
        mu,
        softening: eps,
    } = source;
    // At the center: nothing (and finite); its largest value is at eps/√2.
    let at = gravity_accel(center, &[source]);
    assert!(at.length() < 1e-6, "{at:?}");
    let peak = 2.0 / (3.0 * 3f32.sqrt()) * mu / (eps * eps);
    for k in 1..400 {
        let d = k as f32 * 0.01 * eps * 4.0;
        let a = gravity_accel(center + Vector3::new(d, 0.0, 0.0), &[source]);
        assert!(a.length().is_finite() && a.length() <= peak * 1.0001, "{d}");
    }
    // Far away it is the inverse square.
    let far = 40.0 * eps;
    let a = gravity_accel(center + Vector3::new(far, 0.0, 0.0), &[source]);
    assert!((a.length() / (mu / (far * far)) - 1.0).abs() < 0.01);
}

#[test]
fn boost_into_planet_no_penetration() {
    for dt in [DT, 0.1] {
        for i in 1..=4 {
            let mut s = scene();
            assert!(s.gravity);
            let c = s.bodies[i].transform.position;
            s.ship.position = c + Vector3::new(0.0, 0.0, 6.0 * s.radius_of(i));
            s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
            s.ship.up = Vector3::new(0.0, 1.0, 0.0);
            let boosted = [SolarAction::Forward, SolarAction::Boost];
            let mut closest = f32::INFINITY;
            for _ in 0..(5.0 / dt) as usize {
                step(&mut s, &boosted, &[], Look::default(), dt);
                closest = closest.min(gap(&s, i));
                assert!(s.ship.position.x.is_finite());
            }
            // The collision shape leads the moving planet by up to one
            // frame of its orbit (as in `p3_collision_sweep_tests`): a
            // sliding ship can sit that much inside where the planet is.
            let lead = s.body_velocity(i).length() * dt;
            println!("body {i} dt {dt:.3}: closest gap {closest:.4}, lead {lead:.4}");
            assert!(closest > -(lead + 1e-3), "body {i} dt {dt}: {closest}");
            assert!(closest < 0.5, "body {i} dt {dt}: never reached ({closest})");
        }
    }
}

#[test]
fn local_and_warp_safe() {
    // After a quick travel to each planet, boosting at it with gravity on.
    for i in 1..=4 {
        let mut s = scene();
        assert!(warp_to(&mut s, i));
        for _ in 0..180 {
            step(
                &mut s,
                &[SolarAction::Forward, SolarAction::Boost],
                &[],
                Look::default(),
                DT,
            );
            assert!(gap(&s, i) > -1e-3, "body {i}: {}", gap(&s, i));
        }
    }
    // Local flight over the hearts terraces: the ground holds the ship
    // and gravity changes nothing.
    let path = |gravity: bool| {
        let mut s = scene();
        s.gravity = gravity;
        let i = 3;
        let c = s.bodies[i].transform.position;
        s.ship.position = c + Vector3::new(0.0, s.radius_of(i) * 1.6, 0.0);
        apply_toggles(&mut s, &[SolarAction::ToggleLocal]);
        assert!(matches!(s.flight, FlightMode::Local { .. }));
        let field = s.relief_of(i).unwrap().field;
        (0..240)
            .map(|_| {
                step(
                    &mut s,
                    &[SolarAction::Forward, SolarAction::Descend],
                    &[],
                    Look::default(),
                    DT,
                );
                let c = s.bodies[i].transform.position;
                let d = s.ship.position - c;
                let ground = s.radius_of(i) * field.surface_radius(d.normalized());
                assert!(d.length() > ground, "{} under {ground}", d.length());
                s.ship.position - c
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(path(true), path(false));
}

#[test]
fn fps_equivalence() {
    let end = |fps: f32| {
        let mut s = coasting(true);
        let start = s.ship.position;
        let p = *coast(&mut s, 4.0, 1.0 / fps).last().unwrap();
        (p, (p - start).length())
    };
    let (reference, travelled) = end(60.0);
    for fps in [30.0, 144.0] {
        let (p, _) = end(fps);
        let gap = (p - reference).length();
        assert!(gap < 0.01 * travelled, "{fps} FPS: {gap} of {travelled}");
    }
}

#[test]
fn hud_shows_gravity_state() {
    let mut s = scene();
    assert!(status_line(&s, 0.0, 60.0).ends_with("grav on"));
    assert_eq!(
        apply_toggles(&mut s, &[SolarAction::ToggleGravity]),
        ["Gravity off"]
    );
    assert!(status_line(&s, 0.0, 60.0).ends_with("grav off"));
    assert!(s.gravity_sources().is_empty());
    assert_eq!(
        apply_toggles(&mut s, &[SolarAction::ToggleGravity]),
        ["Gravity on"]
    );
    assert!(status_line(&s, 0.0, 60.0).ends_with("grav on"));
}

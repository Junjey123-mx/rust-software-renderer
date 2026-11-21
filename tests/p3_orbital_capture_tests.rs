//! Orbital capture and tangential escape (Project 3 expansion): a slow
//! ship inside a planet's capture shell with gravity on is held on an
//! assisted orbit; the space bar releases it along the orbit's tangent;
//! the same planet cannot capture it again until it has left the release
//! shell. The assisted orbit of House of Cards stays outside its card ring.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::capture::{CaptureZone, OrbitState};
use rust_software_renderer::solar::flight::{FlightMode, local_frame};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

/// Body indices of the five planets.
const PLANETS: [usize; 5] = [1, 2, 3, 4, 5];
const HOUSE: usize = 4;
const CLOVER: usize = 2;

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.capture.enabled = true;
    s
}

fn zone(s: &SolarScene, i: usize) -> CaptureZone {
    s.capture_zone(i).unwrap()
}

fn distance(s: &SolarScene, i: usize) -> f32 {
    (s.ship.position - s.bodies[i].transform.position).length()
}

/// The ship `d` from planet `i` on the side away from the star, nose along
/// the planet's orbit, drifting towards it at `inward` u/s relative to it
/// (dampers off, so it coasts).
fn place(s: &mut SolarScene, i: usize, d: f32, inward: f32) {
    let p = s.bodies[i].transform.position;
    let n = (p - s.center).normalized();
    s.flight = FlightMode::Space;
    s.ship.position = p + n * d;
    s.ship.forward = Vector3::new(0.0, 1.0, 0.0).cross(n).normalized();
    s.ship.up = n;
    s.ship.renormalize();
    s.ship.velocity = s.body_velocity(i) - n * inward;
    s.ship.dampers = false;
}

fn run(s: &mut SolarScene, actions: &[SolarAction], fps: f32, seconds: f32) {
    for _ in 0..(seconds * fps).round() as usize {
        step(s, actions, &[], Look::default(), 1.0 / fps);
        assert!(s.ship.position.x.is_finite() && s.ship.velocity.x.is_finite());
    }
}

/// Approaches planet `i` slowly and returns once captured (the states it
/// went through are checked on the way).
fn capture(s: &mut SolarScene, i: usize, fps: f32) {
    let z = zone(s, i);
    place(s, i, z.capture + 0.1 * s.radius_of(i), 0.8);
    let mut approached = false;
    for _ in 0..(5.0 * fps) as usize {
        step(s, &[], &[], Look::default(), 1.0 / fps);
        match s.capture.state {
            OrbitState::Approach { planet, .. } => {
                assert_eq!(planet, i);
                approached = true;
            }
            OrbitState::Captured { planet, .. } => {
                assert_eq!(planet, i);
                assert!(approached, "captured without an approach");
                return;
            }
            OrbitState::Free => {}
            other => panic!("{other:?}"),
        }
    }
    panic!("planet {i} at {fps} FPS never captured the ship");
}

#[test]
fn zones_are_ordered_and_clear_the_card_ring() {
    let s = scene();
    assert!(s.capture_zone(0).is_none(), "the star never captures");
    for i in PLANETS {
        let (z, r) = (zone(&s, i), s.radius_of(i));
        let relief = s.relief_of(i).map_or(0.0, |x| x.field.max_height());
        assert!(z.floor >= r * (1.0 + relief) + s.flight_scale(i).clearance - 1e-4);
        assert!(z.floor < z.orbit && z.orbit < z.capture && z.capture < z.release);
    }
    // House of Cards: the lowest assisted orbit leaves the whole ring (and
    // the ship's own radius) below it.
    let ring = s.cards.as_ref().unwrap().extent() * s.radius_of(HOUSE);
    assert!(zone(&s, HOUSE).floor > ring + s.ship.radius(), "{ring}");
}

#[test]
fn f5_quick_travels_to_the_joker() {
    use raylib::consts::KeyboardKey;
    use rust_software_renderer::solar::input::PRESSED;
    assert!(PRESSED.contains(&(KeyboardKey::KEY_F5, SolarAction::Warp(5))));
    let mut s = scene();
    step(
        &mut s,
        &[SolarAction::Warp(5)],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert_eq!(s.bodies[5].name, "Joker");
    assert!(distance(&s, 5) < 3.0 * s.radius_of(5));
}

#[test]
fn disabled_by_default_and_needs_gravity() {
    // Earlier scenes and recordings fly as before.
    let mut s = scene();
    s.capture.enabled = false;
    let z = zone(&s, CLOVER);
    place(&mut s, CLOVER, z.orbit, 0.0);
    run(&mut s, &[], 60.0, 2.0);
    assert_eq!(s.flight, FlightMode::Space);
    assert_eq!(s.capture.state, OrbitState::Free);
    // Gravity off: no capture either.
    let mut s = scene();
    s.gravity = false;
    place(&mut s, CLOVER, z.orbit, 0.0);
    run(&mut s, &[], 60.0, 2.0);
    assert_eq!(s.flight, FlightMode::Space);
}

#[test]
fn light_speed_flyby_is_not_captured() {
    let mut s = scene();
    let z = zone(&s, CLOVER);
    place(&mut s, CLOVER, z.capture, 0.0);
    // Through the shell at boost speed, along the planet's orbit.
    s.ship.velocity = s.body_velocity(CLOVER) + s.ship.forward * 12.0;
    s.ship.boost = 1.0;
    run(
        &mut s,
        &[SolarAction::Forward, SolarAction::Boost],
        60.0,
        1.0,
    );
    assert_eq!(s.flight, FlightMode::Space);
    assert!(!matches!(s.capture.state, OrbitState::Captured { .. }));
}

#[test]
fn every_planet_captures_and_holds_a_lap() {
    for i in PLANETS {
        let mut s = scene();
        capture(&mut s, i, 60.0);
        assert!(matches!(s.flight, FlightMode::Local { planet, .. } if planet == i));
        let label = s.capture_label().unwrap();
        assert_eq!(label, format!("ORBITAL CAPTURE - {}", s.bodies[i].name));
        let z = zone(&s, i);
        let r = s.radius_of(i);
        // A whole lap without touching any key, carried by the moving planet.
        let start_planet = s.bodies[i].transform.position;
        let mut offset = s.ship.position - start_planet;
        let (mut swept, mut frames) = (0.0f32, 0);
        while swept < std::f32::consts::TAU {
            step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
            frames += 1;
            assert!(frames < 60 * 200, "planet {i}: no lap in 200 s ({swept})");
            assert!(matches!(s.capture.state, OrbitState::Captured { .. }));
            let now = s.ship.position - s.bodies[i].transform.position;
            swept += offset
                .normalized()
                .dot(now.normalized())
                .clamp(-1.0, 1.0)
                .acos();
            offset = now;
            let d = now.length();
            assert!(
                d >= z.floor - 1e-3 && d <= z.capture + 1e-3,
                "planet {i}: {d}"
            );
            // Settled on the orbit height after the first seconds.
            if frames > 240 {
                assert!(
                    (d - z.orbit).abs() < 0.05 * r,
                    "planet {i}: {d} vs {}",
                    z.orbit
                );
            }
        }
        let moved = (s.bodies[i].transform.position - start_planet).length();
        assert!(moved > 5.0, "the planet moved {moved}");
    }
}

#[test]
fn house_orbit_never_crosses_the_card_ring() {
    let mut s = scene();
    capture(&mut s, HOUSE, 60.0);
    let ring = s.cards.as_ref().unwrap().extent() * s.radius_of(HOUSE);
    // Even holding Q (down), the assisted orbit keeps above the ring.
    for actions in [&[][..], &[SolarAction::Descend][..]] {
        for _ in 0..600 {
            step(&mut s, actions, &[], Look::default(), 1.0 / 60.0);
            assert!(distance(&s, HOUSE) - s.ship.radius() > ring);
        }
    }
}

#[test]
fn space_escapes_along_the_tangent_without_recapture() {
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    run(&mut s, &[], 60.0, 2.0);
    let center = s.bodies[CLOVER].transform.position;
    let tangent = local_frame(&s.ship, center, s.radius_of(CLOVER))
        .unwrap()
        .tangent;
    let before = s.ship.position;
    step(
        &mut s,
        &[SolarAction::Boost],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert!(matches!(
        s.capture.state,
        OrbitState::Escape { planet: CLOVER, .. }
    ));
    assert_eq!(s.flight, FlightMode::Space);
    assert_eq!(s.capture.blocked, Some(CLOVER));
    // Off along the tangent, carried by the planet: no jump.
    let relative = s.ship.velocity - s.body_velocity(CLOVER);
    assert!(relative.normalized().dot(tangent) > 0.98, "{relative:?}");
    assert!((s.ship.position - before).length() < 0.1);
    // Holding the space bar: the distance keeps growing until the ship is
    // out of the release shell.
    let z = zone(&s, CLOVER);
    let mut d = distance(&s, CLOVER);
    let mut frames = 0;
    while d <= z.release {
        step(
            &mut s,
            &[SolarAction::Boost],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        let now = distance(&s, CLOVER);
        assert!(now >= d - 1e-4, "{d} -> {now}");
        d = now;
        frames += 1;
        assert!(frames < 180, "still inside after 3 s: {d}");
    }
    run(&mut s, &[], 60.0, 0.1);
    assert_eq!(s.capture.blocked, None);
    assert!(!matches!(s.capture.state, OrbitState::Captured { .. }));
}

#[test]
fn released_planet_waits_for_the_ship_to_leave() {
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    // A tap of the space bar, then drifting slowly inside the shell: the
    // planet that released the ship does not take it back.
    step(
        &mut s,
        &[SolarAction::Boost],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    s.ship.dampers = true;
    for _ in 0..180 {
        step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
        assert_eq!(s.flight, FlightMode::Space);
        assert!(!matches!(s.capture.state, OrbitState::Captured { .. }));
        if distance(&s, CLOVER) < zone(&s, CLOVER).release {
            assert_eq!(s.capture.blocked, Some(CLOVER));
        }
    }
    // Once out of the release shell, a new slow approach is captured.
    let mut s2 = scene();
    capture(&mut s2, CLOVER, 60.0);
    step(
        &mut s2,
        &[SolarAction::Boost],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    run(&mut s2, &[SolarAction::Boost], 60.0, 3.0);
    run(&mut s2, &[], 60.0, 0.1);
    assert_eq!(s2.capture.blocked, None);
    capture(&mut s2, CLOVER, 60.0);
}

#[test]
fn holds_and_escapes_at_every_frame_rate() {
    for fps in [10.0, 30.0, 60.0, 144.0] {
        let mut s = scene();
        capture(&mut s, CLOVER, fps);
        let z = zone(&s, CLOVER);
        for _ in 0..(10.0 * fps) as usize {
            step(&mut s, &[], &[], Look::default(), 1.0 / fps);
            let d = distance(&s, CLOVER);
            assert!(
                d >= z.floor - 1e-3 && d <= z.capture + 1e-3,
                "{fps} FPS: {d}"
            );
        }
        let mut d = distance(&s, CLOVER);
        for _ in 0..(1.5 * fps) as usize {
            step(
                &mut s,
                &[SolarAction::Boost],
                &[],
                Look::default(),
                1.0 / fps,
            );
            let now = distance(&s, CLOVER);
            assert!(now >= d - 1e-4, "{fps} FPS: {d} -> {now}");
            d = now;
        }
        assert!(d > z.capture, "{fps} FPS: {d}");
    }
}

#[test]
fn camera_change_keeps_the_capture() {
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    step(
        &mut s,
        &[SolarAction::ToggleCamera],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    run(&mut s, &[], 60.0, 1.0);
    step(
        &mut s,
        &[SolarAction::ToggleCamera],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    run(&mut s, &[], 60.0, 1.0);
    assert!(matches!(
        s.capture.state,
        OrbitState::Captured { planet: CLOVER, .. }
    ));
    let z = zone(&s, CLOVER);
    let d = distance(&s, CLOVER);
    assert!(d >= z.floor - 1e-3 && d <= z.capture + 1e-3, "{d}");
}

#[test]
fn quick_travel_gravity_and_local_end_the_capture() {
    // Quick travel to another planet: free flight there.
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    step(
        &mut s,
        &[SolarAction::Warp(5)],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert_eq!(s.flight, FlightMode::Space);
    assert!(!matches!(s.capture.state, OrbitState::Captured { .. }));
    run(&mut s, &[], 60.0, 0.1);
    assert_ne!(s.capture.blocked, Some(CLOVER), "left behind");
    // Gravity off: the assisted orbit ends where the ship is.
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    let at = s.ship.position - s.bodies[CLOVER].transform.position;
    step(
        &mut s,
        &[SolarAction::ToggleGravity],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert_eq!(s.capture.state, OrbitState::Free);
    let now = s.ship.position - s.bodies[CLOVER].transform.position;
    assert!((now - at).length() < 0.1);
    // L: local exploration around the same planet, no drift without keys.
    let mut s = scene();
    capture(&mut s, CLOVER, 60.0);
    step(
        &mut s,
        &[SolarAction::ToggleLocal],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert!(matches!(s.flight, FlightMode::Local { planet: CLOVER, .. }));
    assert_eq!(s.capture.state, OrbitState::Free);
    let at = s.ship.position - s.bodies[CLOVER].transform.position;
    run(&mut s, &[], 60.0, 1.0);
    let now = s.ship.position - s.bodies[CLOVER].transform.position;
    assert!((now - at).length() < 1e-2, "{at:?} -> {now:?}");
    // L again: space flight, not captured back at once.
    step(
        &mut s,
        &[SolarAction::ToggleLocal],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    run(&mut s, &[], 60.0, 1.0);
    assert!(!matches!(s.capture.state, OrbitState::Captured { .. }));
}

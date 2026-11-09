//! Spacecraft flight controllers and the solar input map.

use raylib::math::Vector3;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::flight::{FlightInput, space_step};
use rust_software_renderer::solar::input::{HELD, PRESSED, SolarAction};

fn close(a: Vector3, b: Vector3, tol: f32) -> bool {
    (a - b).length() < tol
}

fn ship() -> ShipState {
    ShipState::new(Vector3::zero(), 0.1)
}

fn input(actions: &[SolarAction]) -> FlightInput {
    FlightInput::from_actions(actions)
}

#[test]
fn forward_motion_does_not_depend_on_the_frame_rate() {
    let fwd = input(&[SolarAction::Forward]);
    let (mut a, mut b) = (ship(), ship());
    for _ in 0..30 {
        space_step(&mut a, &fwd, 1.0 / 30.0);
    }
    for _ in 0..60 {
        space_step(&mut b, &fwd, 1.0 / 60.0);
    }
    assert!(close(a.position, b.position, 1e-4));
    // One second at move_speed 1.5 along -Z.
    assert!(
        close(a.position, Vector3::new(0.0, 0.0, -1.5), 1e-4),
        "{:?}",
        a.position
    );
}

#[test]
fn turning_does_not_depend_on_the_frame_rate() {
    // Rotations about one fixed axis compose exactly, so each axis alone is
    // frame-rate independent (mixing both is order-dependent by nature).
    for action in [SolarAction::YawLeft, SolarAction::PitchUp] {
        let turn = input(&[action]);
        let (mut a, mut b) = (ship(), ship());
        for _ in 0..30 {
            space_step(&mut a, &turn, 1.0 / 30.0);
        }
        for _ in 0..60 {
            space_step(&mut b, &turn, 1.0 / 60.0);
        }
        assert!(
            close(a.forward, b.forward, 1e-4),
            "{action:?}: {:?} vs {:?}",
            a.forward,
            b.forward
        );
        assert!(close(a.up, b.up, 1e-4));
        assert_eq!(a.position, Vector3::zero(), "turning in place");
        // One second at turn_speed 1 rad/s.
        let angle = a
            .forward
            .dot(Vector3::new(0.0, 0.0, -1.0))
            .clamp(-1.0, 1.0)
            .acos();
        assert!((angle - 1.0).abs() < 1e-3, "{action:?}: {angle}");
    }
}

#[test]
fn opposite_keys_cancel_and_zero_or_bad_deltas_do_nothing() {
    let both = input(&[SolarAction::Forward, SolarAction::Backward]);
    assert_eq!(both.throttle, 0.0);
    let mut s = ship();
    space_step(&mut s, &both, 0.05);
    assert_eq!(s, ship());
    let fwd = input(&[SolarAction::Forward]);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut s = ship();
        space_step(&mut s, &fwd, dt);
        assert_eq!(s.position, Vector3::zero(), "dt {dt}");
    }
    // A stalled frame moves at most one clamped step.
    let mut s = ship();
    space_step(&mut s, &fwd, 5.0);
    assert!((s.position.length() - 0.15).abs() < 1e-5);
}

#[test]
fn a_lifted_nose_travels_the_same_distance_in_its_own_direction() {
    let mut level = ship();
    let mut lifted = ship();
    lifted.pitch(0.6);
    let fwd = input(&[SolarAction::Forward]);
    for _ in 0..10 {
        space_step(&mut level, &fwd, 0.05);
        space_step(&mut lifted, &fwd, 0.05);
    }
    assert!((level.position.length() - lifted.position.length()).abs() < 1e-5);
    assert!(lifted.position.y > 0.4 && level.position.y.abs() < 1e-6);
    assert!(close(lifted.position.normalized(), lifted.forward, 1e-5));
}

#[test]
fn backward_retreats_along_the_nose_and_yaw_left_turns_towards_minus_x() {
    let mut s = ship();
    space_step(&mut s, &input(&[SolarAction::Backward]), 0.1);
    assert!(s.position.z > 0.0);
    let mut s = ship();
    space_step(&mut s, &input(&[SolarAction::YawLeft]), 0.1);
    assert!(s.forward.x < 0.0);
}

#[test]
fn the_input_map_uses_distinct_keys() {
    let mut keys: Vec<_> = HELD.iter().map(|(k, _)| *k as i32).collect();
    keys.extend(PRESSED.iter().map(|(k, _)| *k as i32));
    let n = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), n, "no key bound twice");
}

// ---------------------------------------------------------------------------
// Local frame around a planet
// ---------------------------------------------------------------------------

use rust_software_renderer::solar::flight::local_frame;

#[test]
fn the_local_frame_has_radial_up_distance_altitude_and_a_tangent() {
    let planet = Vector3::new(2.0, 0.0, 0.0);
    let s = ShipState::new(Vector3::new(2.0, 3.0, 0.0), 0.1); // above the planet, nose -Z
    let f = local_frame(&s, planet, 1.0).unwrap();
    assert!(close(f.normal, Vector3::new(0.0, 1.0, 0.0), 1e-6));
    assert!((f.distance - 3.0).abs() < 1e-6 && (f.altitude - 2.0).abs() < 1e-6);
    assert!(close(f.tangent, Vector3::new(0.0, 0.0, -1.0), 1e-6));
    assert!(f.tangent.dot(f.normal).abs() < 1e-6);
}

#[test]
fn below_the_planet_the_local_up_points_down() {
    let s = ShipState::new(Vector3::new(0.0, -2.0, 0.0), 0.1);
    let f = local_frame(&s, Vector3::zero(), 0.5).unwrap();
    assert!(close(f.normal, Vector3::new(0.0, -1.0, 0.0), 1e-6));
    assert!(f.tangent.dot(f.normal).abs() < 1e-6);
}

#[test]
fn a_radial_nose_falls_back_to_the_projected_up() {
    // Ship on +Z of the planet pointing straight at it (nose -Z).
    let s = ShipState::new(Vector3::new(0.0, 0.0, 3.0), 0.1);
    let f = local_frame(&s, Vector3::zero(), 1.0).unwrap();
    assert!(close(f.tangent, s.up, 1e-6), "{:?}", f.tangent);
    // Tilted slightly off radial: the projected nose is used and stays unit.
    let mut t = s;
    t.pitch(0.01);
    let g = local_frame(&t, Vector3::zero(), 1.0).unwrap();
    assert!((g.tangent.length() - 1.0).abs() < 1e-4);
    assert!(g.tangent.dot(g.normal).abs() < 1e-4);
}

#[test]
fn a_ship_at_the_center_or_non_finite_values_have_no_frame() {
    let s = ShipState::new(Vector3::zero(), 0.1);
    assert!(local_frame(&s, Vector3::zero(), 1.0).is_none());
    let s = ShipState::new(Vector3::new(0.0, 0.0, f32::NAN), 0.1);
    assert!(local_frame(&s, Vector3::zero(), 1.0).is_none());
    let s = ShipState::new(Vector3::new(0.0, 2.0, 0.0), 0.1);
    assert!(local_frame(&s, Vector3::zero(), f32::INFINITY).is_none());
}

#[test]
fn the_frame_holds_near_the_poles_and_at_any_heading() {
    let mut s = ShipState::new(Vector3::new(0.0, 5.0, 0.001), 0.1);
    for i in 0..36 {
        s.yaw(0.17);
        if i % 4 == 0 {
            s.pitch(0.3);
        }
        let f = local_frame(&s, Vector3::zero(), 1.0).unwrap();
        assert!((f.tangent.length() - 1.0).abs() < 1e-4);
        assert!(f.tangent.dot(f.normal).abs() < 1e-4);
    }
}

// ---------------------------------------------------------------------------
// FlightMode and the L toggle
// ---------------------------------------------------------------------------

use rust_software_renderer::solar::flight::{
    CLEARANCE, FlightMode, LOCAL_RANGE, LocalTarget, toggle_local,
};

fn targets() -> Vec<LocalTarget> {
    vec![
        LocalTarget {
            index: 1,
            position: Vector3::new(3.0, 0.0, 0.0),
            radius: 0.5,
        },
        LocalTarget {
            index: 2,
            position: Vector3::new(-4.0, 0.0, 0.0),
            radius: 0.8,
        },
    ]
}

#[test]
fn entering_local_flight_picks_the_nearest_planet_and_records_its_position() {
    let s = ShipState::new(Vector3::new(3.0, 1.5, 0.0), 0.1);
    let mode = toggle_local(FlightMode::Space, &s, &targets()).unwrap();
    assert_eq!(
        mode,
        FlightMode::Local {
            planet: 1,
            previous_planet_position: Vector3::new(3.0, 0.0, 0.0),
        }
    );
    assert_eq!(mode.name(), "local");
    // Leaving returns to space flight; the ship is untouched by both toggles.
    assert_eq!(
        toggle_local(mode, &s, &targets()).unwrap(),
        FlightMode::Space
    );
}

#[test]
fn entry_is_refused_inside_the_clearance_and_too_far_away() {
    let inside = ShipState::new(Vector3::new(3.0, 0.5 + CLEARANCE * 0.5, 0.0), 0.1);
    assert!(
        toggle_local(FlightMode::Space, &inside, &targets())
            .unwrap_err()
            .contains("too close")
    );
    let center = ShipState::new(Vector3::new(3.0, 0.0, 0.0), 0.1);
    assert!(toggle_local(FlightMode::Space, &center, &targets()).is_err());
    let far = ShipState::new(Vector3::new(3.0, 0.5 + LOCAL_RANGE + 1.0, 0.0), 0.1);
    assert!(
        toggle_local(FlightMode::Space, &far, &targets())
            .unwrap_err()
            .contains("too far")
    );
    assert!(toggle_local(FlightMode::Space, &far, &[]).is_err());
}

#[test]
fn the_scene_never_offers_the_star_and_toggling_keeps_the_ship_in_place() {
    use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
    use rust_software_renderer::solar::{SolarMeshes, SolarScene};
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        5.0,
        Viewport::new(320, 240, 0.1),
    )
    .unwrap();
    let mut scene = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    let targets = scene.local_targets();
    assert_eq!(targets.iter().map(|t| t.index).collect::<Vec<_>>(), [1, 2]);
    // Park the ship 1 unit above Crimson Entrance (index 1).
    let p = scene.bodies[1].transform.position;
    scene.ship.position = p + Vector3::new(0.0, scene.radius_of(1) + 1.0, 0.0);
    let before = scene.ship;
    assert!(matches!(
        scene.toggle_local().unwrap(),
        FlightMode::Local { planet: 1, .. }
    ));
    assert_eq!(scene.ship, before);
    assert_eq!(scene.toggle_local().unwrap(), FlightMode::Space);
    assert_eq!(scene.ship, before);
    // From the spawn, outside every range: refused, still in space flight.
    scene.ship.position = Vector3::new(0.0, 0.6, 7.5);
    assert!(scene.toggle_local().is_err());
    assert_eq!(scene.flight, FlightMode::Space);
}

//! Ship attitude in the live viewer (second pass): the ship banks into
//! turns and strafes and comes back to level, only in its drawing; near a
//! planet its "up" levels with the local vertical, and while captured the
//! nose settles on the tangent plane, smoothly and without flips.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::{
    BANK_EASE, FlightInput, MAX_BANK, bank_target, ease_bank,
};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

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

#[test]
fn bank_leans_into_strafes_and_turns() {
    let strafe_right = FlightInput {
        strafe: 1.0,
        ..FlightInput::default()
    };
    let turn_left = FlightInput {
        yaw: 1.0,
        ..FlightInput::default()
    };
    assert!(bank_target(&strafe_right, 1.0 / 60.0) > 0.3);
    assert!(bank_target(&turn_left, 1.0 / 60.0) < -0.3);
    let mouse_right = FlightInput {
        look: Look {
            yaw: -0.2,
            pitch: 0.0,
        },
        ..FlightInput::default()
    };
    assert_eq!(bank_target(&mouse_right, 1.0 / 60.0), MAX_BANK);
    assert_eq!(bank_target(&FlightInput::default(), 1.0 / 60.0), 0.0);
    let nan = FlightInput {
        strafe: f32::NAN,
        ..FlightInput::default()
    };
    assert_eq!(bank_target(&nan, 1.0 / 60.0), 0.0);
}

#[test]
fn bank_eases_in_and_back_to_level() {
    let mut b = 0.0;
    for _ in 0..60 {
        b = ease_bank(b, MAX_BANK, 1.0 / 60.0);
    }
    assert!(b > 0.95 * MAX_BANK && b <= MAX_BANK, "{b}");
    // Never a jump: one frame moves it a share of the gap.
    let one = ease_bank(0.0, MAX_BANK, 1.0 / 60.0);
    assert!(one < 0.1 * MAX_BANK, "{one}");
    for _ in 0..(1.5 * 60.0) as usize {
        b = ease_bank(b, 0.0, 1.0 / 60.0);
    }
    assert!(b.abs() < 0.02, "{b}");
    const { assert!(BANK_EASE > 0.1 && BANK_EASE < 0.5) };
    assert_eq!(ease_bank(f32::NAN, 0.3, 0.1), 0.0);
    assert_eq!(ease_bank(0.2, 0.3, 0.0), 0.2);
}

#[test]
fn bank_rolls_only_the_drawing() {
    let mut s = scene();
    let mut plain = s.clone();
    for _ in 0..30 {
        step(
            &mut s,
            &[SolarAction::StrafeRight],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        // A run whose bank is wiped every frame flies the same.
        plain.bank = 0.0;
        step(
            &mut plain,
            &[SolarAction::StrafeRight],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
    }
    assert!(s.bank > 0.3, "{}", s.bank);
    // The bank never feeds back into the flight, and the drawn ship is the
    // flight ship rolled about its nose.
    assert_eq!(s.ship, plain.ship);
    let drawn = s.drawn_ship();
    assert_eq!(drawn.position, s.ship.position);
    assert!((drawn.forward - s.ship.forward).length() < 1e-5);
    assert!((drawn.up.dot(s.ship.up) - s.bank.cos()).abs() < 1e-4);
    // Right wing down: the drawn top leans towards the right.
    assert!(drawn.up.dot(s.ship.right()) > 0.2);
}

#[test]
fn recordings_and_tests_without_the_live_controller_never_bank() {
    let mut s = scene();
    s.capture.enabled = false;
    for _ in 0..30 {
        step(
            &mut s,
            &[SolarAction::StrafeRight],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
    }
    assert_eq!(s.bank, 0.0);
    assert_eq!(s.drawn_ship(), s.ship);
}

#[test]
fn captured_ship_levels_with_the_planet_without_jumps() {
    use rust_software_renderer::solar::capture::OrbitState;
    use rust_software_renderer::solar::flight::FlightMode;
    let mut s = scene();
    let i = 2;
    let z = s.capture_zone(i).unwrap();
    let p = s.bodies[i].transform.position;
    let n = (p - s.center).normalized();
    s.flight = FlightMode::Space;
    s.ship.position = p + n * (z.capture + 0.1 * s.radius_of(i));
    // Rolled a quarter turn off the planet's vertical: "up" along the
    // world's up, square to the planet's normal.
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s.ship.forward = s.ship.up.cross(n).normalized();
    s.ship.renormalize();
    s.ship.velocity = s.body_velocity(i) - n * 0.8;
    s.ship.dampers = false;
    let start = s.ship.up;
    assert!(start.dot(n).abs() < 0.05);
    let mut previous = s.ship.up;
    let mut largest_turn = 0.0f32;
    for _ in 0..(6 * 60) {
        step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
        largest_turn = largest_turn.max(previous.dot(s.ship.up).clamp(-1.0, 1.0).acos());
        previous = s.ship.up;
    }
    assert!(matches!(s.capture.state, OrbitState::Captured { planet, .. } if planet == i));
    let normal = (s.ship.position - s.bodies[i].transform.position).normalized();
    assert!(s.ship.up.dot(normal) > 0.97, "{}", s.ship.up.dot(normal));
    assert!(
        s.ship.forward.dot(normal).abs() < 0.1,
        "{}",
        s.ship.forward.dot(normal)
    );
    // At 60 FPS the attitude never turns more than about 3 degrees a frame.
    assert!(largest_turn < 0.05, "{largest_turn}");
}

#[test]
fn without_the_live_controller_the_attitude_is_left_alone() {
    use rust_software_renderer::solar::flight::FlightMode;
    let mut s = scene();
    s.capture.enabled = false;
    let i = 2;
    let z = s.capture_zone(i).unwrap();
    let p = s.bodies[i].transform.position;
    let n = (p - s.center).normalized();
    s.flight = FlightMode::Space;
    s.ship.position = p + n * (z.capture + 0.1 * s.radius_of(i));
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s.ship.forward = s.ship.up.cross(n).normalized();
    s.ship.renormalize();
    for _ in 0..60 {
        step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
    }
    assert!((s.ship.up - Vector3::new(0.0, 1.0, 0.0)).length() < 1e-3);
}

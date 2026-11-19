//! Quick travel: warp next to a body at its current position.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::{CLEARANCE, FlightMode};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::travel::{WARP_MARGIN, arrival_point, warp_to};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        5.3,
        Viewport::new(320, 240, 0.1),
    )
    .unwrap();
    SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

#[test]
fn warping_lands_outside_the_destination_facing_it_in_space_flight() {
    for index in 0..3 {
        let mut s = scene();
        s.clock.time = 7.3;
        s.update();
        assert!(warp_to(&mut s, index));
        let target = s.bodies[index].transform.position;
        let d = (s.ship.position - target).length();
        assert!(
            (d - (s.radius_of(index) + WARP_MARGIN)).abs() < 1e-4,
            "{index}: {d}"
        );
        assert!(s.ship.forward.dot((target - s.ship.position).normalized()) > 0.9999);
        assert!(s.ship.forward.dot(s.ship.up).abs() < 1e-5);
        assert_eq!(s.flight, FlightMode::Space);
        // Never inside another body.
        for (i, b) in s.bodies.iter().enumerate() {
            if i != index {
                assert!(
                    (s.ship.position - b.transform.position).length() > s.radius_of(i) + CLEARANCE
                );
            }
        }
    }
}

#[test]
fn warping_uses_the_current_orbital_position_and_does_not_touch_the_orbits() {
    let mut s = scene();
    s.clock.time = 3.0;
    s.update();
    let bodies: Vec<_> = s.bodies.iter().map(|b| b.transform).collect();
    let time = s.time();
    warp_to(&mut s, 1);
    assert_eq!(
        bodies,
        s.bodies.iter().map(|b| b.transform).collect::<Vec<_>>()
    );
    assert_eq!(s.time(), time);
    let at_t3 = s.ship.position;
    let mut later = scene();
    later.clock.time = 9.0;
    later.update();
    warp_to(&mut later, 1);
    assert!(
        (later.ship.position - at_t3).length() > 0.5,
        "follows the orbit"
    );
}

#[test]
fn warping_leaves_local_flight_and_local_flight_can_start_again() {
    let mut s = scene();
    warp_to(&mut s, 2);
    assert!(matches!(
        s.toggle_local().unwrap(),
        FlightMode::Local { planet: 2, .. }
    ));
    warp_to(&mut s, 1);
    assert_eq!(s.flight, FlightMode::Space);
    assert!(matches!(
        s.toggle_local().unwrap(),
        FlightMode::Local { planet: 1, .. }
    ));
}

#[test]
fn a_blocked_approach_side_picks_another_clear_arrival_point() {
    let mut s = scene();
    s.update();
    // Put the ship straight behind Entrance as seen from the star: arriving
    // on that side is clear, but place the star in the way of a ship coming
    // from the far side by putting the ship beyond the star.
    let e = s.bodies[1].transform.position;
    s.ship.position = e * -3.0;
    let p = arrival_point(&s, 1).unwrap();
    let star_gap = (p - s.bodies[0].transform.position).length();
    assert!(star_gap > s.radius_of(0) + CLEARANCE, "{star_gap}");
    assert!(arrival_point(&s, 9).is_none());
    assert!(!warp_to(&mut s, 9));
}

#[test]
fn function_keys_warp_through_the_viewer_step_deterministically() {
    let mut a = scene();
    let mut b = scene();
    step(&mut a, &[SolarAction::Warp(2)], &[], Look::default(), 0.0);
    step(&mut b, &[SolarAction::Warp(2)], &[], Look::default(), 0.0);
    assert_eq!(a.ship, b.ship);
    let target = a.bodies[2].transform.position;
    assert!(((a.ship.position - target).length() - (a.radius_of(2) + WARP_MARGIN)).abs() < 1e-4);
}

//! Camera rigs: follow camera behind the spacecraft and the orbit rig.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, ViewPreset};
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::camera::{CameraRig, FollowRig, orbit_perspective};
use rust_software_renderer::solar::flight::{FlightInput, space_step};
use rust_software_renderer::solar::input::SolarAction;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-4
}

#[test]
fn the_follow_camera_sits_behind_and_above_and_looks_ahead() {
    let ship = ShipState::new(Vector3::new(1.0, 2.0, 3.0), 0.08);
    let rig = FollowRig::default();
    let cam = rig.camera(&ship);
    assert!(close(
        cam.eye,
        Vector3::new(1.0, 2.0 + rig.height, 3.0 + rig.distance)
    ));
    assert!(close(
        cam.target,
        Vector3::new(1.0, 2.0, 3.0 - rig.look_ahead)
    ));
    assert!(close(cam.up, ship.up));
    // The ship is in front of the camera, between eye and target.
    let v = cam.world_to_view(ship.position);
    assert!(v.z > cam.near && v.z < (cam.target - cam.eye).length());
}

#[test]
fn moving_and_turning_the_ship_carry_the_camera_at_the_same_offset() {
    let rig = FollowRig::default();
    let mut ship = ShipState::new(Vector3::zero(), 0.08);
    let offset = |s: &ShipState| (rig.camera(s).eye - s.position).length();
    let before = offset(&ship);
    let fly = FlightInput::from_actions(&[SolarAction::Forward, SolarAction::YawLeft]);
    for _ in 0..40 {
        space_step(&mut ship, &fly, 0.05);
    }
    assert!((offset(&ship) - before).abs() < 1e-4);
    let cam = rig.camera(&ship);
    assert!(
        (cam.eye - (ship.position - ship.forward * rig.distance + ship.up * rig.height)).length()
            < 1e-4
    );
    assert!(
        cam.basis().forward.dot(ship.forward) > 0.9,
        "looks where the ship flies"
    );
}

#[test]
fn changing_the_follow_distance_frames_differently_without_moving_the_ship() {
    let ship = ShipState::new(Vector3::new(0.0, 0.0, 5.0), 0.08);
    let near = FollowRig::default().camera(&ship);
    let far = FollowRig {
        distance: 2.0,
        ..FollowRig::default()
    }
    .camera(&ship);
    assert!(far.world_to_view(ship.position).z > near.world_to_view(ship.position).z);
    assert_eq!(ship.position, Vector3::new(0.0, 0.0, 5.0));
}

#[test]
fn the_orbit_rig_frames_the_target_and_zoom_approaches() {
    let mut orbit = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    let a = orbit_perspective(&orbit, 12.0);
    assert!(close(a.target, Vector3::zero()));
    assert!((a.eye.length() - 12.0).abs() < 1e-4);
    orbit.zoom_by(2.0);
    assert!((orbit_perspective(&orbit, 12.0).eye.length() - 6.0).abs() < 1e-4);
    assert!(a.fov_y > 0.0 && a.near > 0.0);
}

#[test]
fn rigs_toggle_and_follow_is_the_default() {
    assert_eq!(CameraRig::default(), CameraRig::Follow);
    assert_eq!(CameraRig::Follow.toggled(), CameraRig::Orbit);
    assert_eq!(CameraRig::Orbit.toggled().name(), "follow");
}

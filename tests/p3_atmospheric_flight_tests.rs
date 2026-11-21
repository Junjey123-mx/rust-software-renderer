//! Atmospheric flight and the camera recovery of the live viewer (Grand
//! scale): a ship that dives into a planet's atmosphere at any speed is
//! taken into a flight that follows the ground at a target height without
//! pushing it out; the turbo leaves along the tangent; back in space the
//! ship's roll and the follow framing return to the space ones once, on a
//! schedule that ends the same at any frame rate, without touching the
//! position, the velocity or the nose.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::camera::{ATMOSPHERE_RIG, FollowRig};
use rust_software_renderer::solar::capture::{
    CameraRecovery, OrbitState, RECOVERY_SECONDS, SPACE_UP,
};
use rust_software_renderer::solar::flight::FlightMode;
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const CLOVER: usize = 2;

fn grand() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::grand_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.capture.enabled = true;
    s
}

fn run(s: &mut SolarScene, actions: &[SolarAction], fps: f32, seconds: f32) {
    for _ in 0..(seconds * fps).round() as usize {
        step(s, actions, &[], Look::default(), 1.0 / fps);
        assert!(s.ship.position.x.is_finite() && s.ship.up.x.is_finite());
    }
}

/// The ship diving fast at planet `i` from just outside its atmosphere.
fn dive(s: &mut SolarScene, i: usize) {
    let c = s.bodies[i].transform.position;
    let n = (c - s.center).normalized();
    s.ship.position = c + n * (s.atmosphere_top(i) + 0.5);
    s.ship.forward = Vector3::new(0.0, 1.0, 0.0).cross(n).normalized();
    s.ship.up = n;
    s.ship.renormalize();
    s.ship.velocity = s.body_velocity(i) - n * 8.0;
}

fn height(s: &SolarScene, i: usize) -> f32 {
    (s.ship.position - s.bodies[i].transform.position).length() - s.ground_below(i)
}

#[test]
fn a_fast_dive_enters_the_atmosphere_and_settles_low() {
    let mut s = grand();
    dive(&mut s, CLOVER);
    run(&mut s, &[], 60.0, 1.0);
    assert!(matches!(s.flight, FlightMode::Local { planet: CLOVER, .. }));
    let target = s.surface_altitude.expect("atmospheric flight");
    let clearance = s.flight_scale(CLOVER).clearance;
    assert!(
        (clearance..=1.5 * clearance + 1e-3).contains(&target),
        "{target}"
    );
    run(&mut s, &[], 60.0, 4.0);
    assert!(
        (height(&s, CLOVER) - target).abs() < 0.1,
        "{}",
        height(&s, CLOVER)
    );
}

#[test]
fn flying_over_the_relief_keeps_the_height_without_drifting_out() {
    let mut s = grand();
    dive(&mut s, CLOVER);
    run(&mut s, &[], 60.0, 5.0);
    let target = s.surface_altitude.unwrap();
    let clearance = s.flight_scale(CLOVER).clearance;
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for _ in 0..(20 * 60) {
        step(
            &mut s,
            &[SolarAction::Forward],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        let h = height(&s, CLOVER);
        assert!(h >= clearance - 1e-3, "below the clearance: {h}");
        low = low.min(h);
        high = high.max(h);
    }
    assert!(matches!(s.flight, FlightMode::Local { planet: CLOVER, .. }));
    // Over mountains and sea it stays close to the target: never ratcheted
    // outwards by the relief.
    assert!(
        high < target + 1.5 && low > target - 1.0,
        "{low} {high} {target}"
    );
    assert!((height(&s, CLOVER) - target).abs() < 1.0);
}

#[test]
fn e_and_q_move_the_target_within_the_atmosphere() {
    let mut s = grand();
    dive(&mut s, CLOVER);
    run(&mut s, &[], 60.0, 2.0);
    let low = s.surface_altitude.unwrap();
    run(&mut s, &[SolarAction::Ascend], 60.0, 3.0);
    let up = s.surface_altitude.unwrap();
    assert!(up > low);
    let ceiling = s.atmosphere_top(CLOVER) - s.radius_of(CLOVER);
    assert!(up <= ceiling + 1e-3);
    run(&mut s, &[SolarAction::Descend], 60.0, 10.0);
    assert!((s.surface_altitude.unwrap() - s.flight_scale(CLOVER).clearance).abs() < 1e-3);
}

#[test]
fn the_turbo_leaves_along_the_tangent_and_the_view_recovers() {
    let mut s = grand();
    dive(&mut s, CLOVER);
    run(&mut s, &[], 60.0, 3.0);
    // A rolled, inverted attitude on the way out.
    s.ship.rotate(s.ship.forward, 2.5);
    let before = s.ship.position;
    step(
        &mut s,
        &[SolarAction::Boost],
        &[],
        Look::default(),
        1.0 / 60.0,
    );
    assert_eq!(s.flight, FlightMode::Space);
    assert!(matches!(
        s.capture.state,
        OrbitState::Escape { planet: CLOVER, .. }
    ));
    assert!((s.ship.position - before).length() < 1.0, "no teleport");
    assert!(s.camera_recovery.is_some());
    let nose = s.ship.forward;
    let d0 = (s.ship.position - s.bodies[CLOVER].transform.position).length();
    run(&mut s, &[SolarAction::Boost], 60.0, RECOVERY_SECONDS + 0.1);
    let d1 = (s.ship.position - s.bodies[CLOVER].transform.position).length();
    assert!(d1 > d0, "{d0} -> {d1}");
    // The nose kept its heading (no flip), the roll is the space one and
    // the framing is the canonical follow rig.
    assert!(s.ship.forward.dot(nose) > 0.9);
    let want = (SPACE_UP - s.ship.forward * SPACE_UP.dot(s.ship.forward)).normalized();
    assert!(s.ship.up.dot(want) > 0.9999, "{:?}", s.ship.up);
    assert_eq!(s.camera_recovery, None);
    assert_eq!(s.atmosphere_blend, 0.0);
}

/// A ship that has just left a planet with this roll about its nose.
fn leaving(roll: f32, from_blend: f32) -> SolarScene {
    let mut s = grand();
    s.ship.position = Vector3::new(40.0, -12.0, 30.0);
    s.ship.forward = Vector3::new(0.6, -0.3, 0.74).normalized();
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s.ship.renormalize();
    s.ship.rotate(s.ship.forward, roll);
    s.ship.velocity = Vector3::new(3.0, 1.0, -2.0);
    s.atmosphere_blend = from_blend;
    s.camera_recovery = Some(CameraRecovery {
        progress: 0.0,
        from_blend,
    });
    s
}

#[test]
fn recovery_ends_the_same_at_any_frame_rate_and_hemisphere() {
    for roll in [-3.0f32, -1.2, 0.4, 2.9] {
        let mut ups = Vec::new();
        for fps in [10.0f32, 30.0, 60.0, 144.0] {
            let mut s = leaving(roll, 1.0);
            let (p, v, f) = (s.ship.position, s.ship.velocity, s.ship.forward);
            let mut last = s.atmosphere_blend;
            for _ in 0..(RECOVERY_SECONDS * fps).ceil() as usize {
                s.recover_space_view(1.0 / fps);
                assert!(s.ship.up.x.is_finite() && s.atmosphere_blend <= last + 1e-6);
                last = s.atmosphere_blend;
            }
            assert_eq!(s.camera_recovery, None, "ends once, {fps} fps");
            assert_eq!(s.atmosphere_blend, 0.0);
            // Position, velocity and nose untouched.
            assert_eq!((s.ship.position, s.ship.velocity), (p, v));
            assert!((s.ship.forward - f).length() < 1e-4);
            ups.push(s.ship.up);
        }
        let n = leaving(roll, 1.0).ship.forward;
        let want = (SPACE_UP - n * SPACE_UP.dot(n)).normalized();
        for up in &ups {
            assert!(up.dot(want) > 0.9999, "roll {roll}: {up:?}");
        }
    }
}

#[test]
fn recovery_is_safe_with_the_nose_along_the_vertical() {
    let mut s = leaving(0.0, 0.5);
    s.ship.forward = SPACE_UP;
    s.ship.up = Vector3::new(0.0, 0.0, 1.0);
    s.ship.renormalize();
    for _ in 0..120 {
        s.recover_space_view(1.0 / 60.0);
    }
    assert!(s.ship.up.x.is_finite() && s.ship.forward.x.is_finite());
    assert_eq!(s.atmosphere_blend, 0.0);
}

#[test]
fn follow_rig_is_unchanged_outside_the_atmosphere() {
    let rig = FollowRig::default();
    assert_eq!(rig.blended(&ATMOSPHERE_RIG, 0.0), rig);
    assert_eq!(rig.blended(&ATMOSPHERE_RIG, f32::NAN), rig);
    assert_eq!(rig.blended(&ATMOSPHERE_RIG, 1.0), ATMOSPHERE_RIG);
    // Scenes without the live controller never blend or recover.
    let mut s = grand();
    s.capture.enabled = false;
    dive(&mut s, CLOVER);
    let up = s.ship.up;
    s.recover_space_view(1.0 / 60.0);
    assert_eq!((s.atmosphere_blend, s.ship.up), (0.0, up));
}

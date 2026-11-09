//! Orbit: closed-form circular motion on XZ.

use raylib::math::Vector3;
use rust_software_renderer::solar::Orbit;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-4
}

#[test]
fn angle_zero_is_plus_x_and_quarter_turn_is_plus_z() {
    let o = Orbit::new(2.5, 0.5, 0.0).unwrap();
    assert!(close(
        o.position_at(0.0, Vector3::zero()),
        Vector3::new(2.5, 0.0, 0.0)
    ));
    let quarter = FRAC_PI_2 / 0.5;
    assert!(close(
        o.position_at(quarter, Vector3::zero()),
        Vector3::new(0.0, 0.0, 2.5)
    ));
    assert!(close(
        o.position_at(PI / 0.5, Vector3::zero()),
        Vector3::new(-2.5, 0.0, 0.0)
    ));
}

#[test]
fn radius_is_constant_and_the_center_is_added() {
    let o = Orbit::new(3.0, 0.7, 1.1).unwrap();
    let center = Vector3::new(1.0, 2.0, 3.0);
    for i in 0..100 {
        let p = o.position_at(i as f32 * 0.37, center);
        assert!(((p - center).length() - 3.0).abs() < 1e-4);
        assert_eq!(p.y, center.y);
    }
}

#[test]
fn phase_shifts_the_start_and_speed_sets_the_period() {
    let o = Orbit::new(1.0, 0.5, FRAC_PI_2).unwrap();
    assert!(close(
        o.position_at(0.0, Vector3::zero()),
        Vector3::new(0.0, 0.0, 1.0)
    ));
    assert!((o.period() - TAU / 0.5).abs() < 1e-5);
    assert!(close(
        o.position_at(o.period(), Vector3::zero()),
        o.position_at(0.0, Vector3::zero())
    ));
    let backwards = Orbit::new(1.0, -0.5, 0.0).unwrap();
    assert!(backwards.position_at(0.1, Vector3::zero()).z < 0.0);
}

#[test]
fn large_times_stay_on_the_circle_and_bad_parameters_are_rejected() {
    let o = Orbit::new(4.4, 0.3, 0.0).unwrap();
    let p = o.position_at(1.0e4, Vector3::zero());
    assert!((p.length() - 4.4).abs() < 1e-2);
    assert!(Orbit::new(0.0, 1.0, 0.0).is_err());
    assert!(Orbit::new(-1.0, 1.0, 0.0).is_err());
    assert!(Orbit::new(f32::NAN, 1.0, 0.0).is_err());
    assert!(Orbit::new(1.0, f32::INFINITY, 0.0).is_err());
}

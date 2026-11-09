//! SimulationClock: pause, speed and step clamping.

use rust_software_renderer::solar::{MAX_STEP, SimulationClock};

#[test]
fn advances_by_the_real_delta_at_speed_one() {
    let mut c = SimulationClock::new(2.0);
    c.advance(1.0 / 60.0);
    assert!((c.time - (2.0 + 1.0 / 60.0)).abs() < 1e-6);
    assert!((c.delta - 1.0 / 60.0).abs() < 1e-6);
    assert_eq!(c.real_delta, c.delta);
}

#[test]
fn pause_freezes_simulated_time_but_keeps_the_real_delta() {
    let mut c = SimulationClock::default();
    assert!(c.toggle_pause());
    c.advance(0.05);
    assert_eq!(c.time, 0.0);
    assert_eq!(c.delta, 0.0);
    assert_eq!(c.real_delta, 0.05);
    assert!(!c.toggle_pause());
    c.advance(0.05);
    assert_eq!(c.time, 0.05);
}

#[test]
fn speed_scales_the_simulated_step_and_rejects_bad_values() {
    let mut c = SimulationClock::default();
    c.set_speed(2.0);
    c.advance(0.05);
    assert!((c.time - 0.1).abs() < 1e-6);
    assert_eq!(c.real_delta, 0.05, "real time is never scaled");
    c.set_speed(-1.0);
    c.set_speed(f32::NAN);
    assert_eq!(c.speed, 2.0);
    c.set_speed(0.0);
    c.advance(0.05);
    assert!((c.time - 0.1).abs() < 1e-6, "speed 0 stands still");
}

#[test]
fn huge_zero_and_non_finite_deltas_are_clamped() {
    let mut c = SimulationClock::default();
    c.advance(5.0);
    assert_eq!(c.time, MAX_STEP);
    c.advance(0.0);
    assert_eq!(c.time, MAX_STEP);
    c.advance(f32::NAN);
    c.advance(f32::INFINITY);
    c.advance(-1.0);
    assert_eq!(c.time, MAX_STEP);
    assert_eq!(c.delta, 0.0);
}

#[test]
fn the_same_deltas_give_the_same_time_regardless_of_jitter_sum() {
    let mut a = SimulationClock::default();
    let mut b = SimulationClock::default();
    for _ in 0..60 {
        a.advance(1.0 / 60.0);
    }
    for _ in 0..30 {
        b.advance(1.0 / 30.0);
    }
    assert!((a.time - b.time).abs() < 1e-4);
}

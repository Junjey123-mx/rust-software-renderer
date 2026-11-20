//! Project 3 clover sea in motion: waves and foam move with the simulated
//! time, repeat at a fixed time, freeze with the pause and stay finite at
//! any time (wrapped into the common wave period).

use raylib::math::Vector3;
use rust_software_renderer::solar::SimulationClock;
use rust_software_renderer::solar::planet::clover::{
    CloverParams, FOAM_BAND, PreparedClover, WAVE_PERIOD, foam, wave_normal,
};

fn prepared(time: f32, params: CloverParams) -> PreparedClover {
    PreparedClover {
        field: params.field(),
        params,
        light: Vector3::new(-0.4, 0.7, 0.6).normalized(),
        time,
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Fingerprint of the material on an orthographic disc facing clover 0
/// (sea, coast and land in view).
fn disc_hash(p: &PreparedClover) -> u64 {
    let size = 96;
    let c = p.field.centers[0];
    let right = c.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let up = right.cross(c);
    let mut bytes = Vec::new();
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let v = 1.0 - (y as f32 + 0.5) / size as f32 * 2.0;
            let r2 = u * u + v * v;
            if r2 >= 1.0 {
                continue;
            }
            let n = right * u + up * v + c * (1.0 - r2).sqrt();
            let color = p.shade(n, c);
            for ch in [color.x, color.y, color.z] {
                bytes.push((ch.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    fnv1a64(&bytes)
}

#[test]
fn hash_changes_with_time() {
    let params = CloverParams::default();
    let a = disc_hash(&prepared(0.0, params));
    let b = disc_hash(&prepared(1.5, params));
    assert_ne!(a, b);
    // Still water (speed 0) only moves its foam: with no foam it is static.
    let mut still = params;
    still.wave_speed = 0.0;
    still.foam = 0.0;
    assert_eq!(
        disc_hash(&prepared(0.0, still)),
        disc_hash(&prepared(5.0, still))
    );
}

#[test]
fn hash_repeats_at_fixed_time() {
    let params = CloverParams::default();
    let a = disc_hash(&prepared(3.25, params));
    assert_eq!(a, disc_hash(&prepared(3.25, params)));
    // One whole period later, the same waves and foam.
    let b = disc_hash(&prepared(3.0, params));
    assert_eq!(b, disc_hash(&prepared(3.0 + WAVE_PERIOD, params)));
}

#[test]
fn paused_is_frozen() {
    let params = CloverParams::default();
    let mut clock = SimulationClock::new(12.0);
    clock.advance(0.1);
    let before = disc_hash(&prepared(clock.time, params));
    clock.toggle_pause();
    for _ in 0..10 {
        clock.advance(0.1);
    }
    assert_eq!(disc_hash(&prepared(clock.time, params)), before);
    clock.toggle_pause();
    clock.advance(0.5);
    assert_ne!(disc_hash(&prepared(clock.time, params)), before);
}

#[test]
fn no_nan_wave_normals() {
    let params = CloverParams::default();
    for i in 0..500 {
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / 500.0;
        let r = (1.0 - y * y).sqrt();
        let a = i as f32 * 2.399_963;
        let n = Vector3::new(r * a.cos(), y, r * a.sin());
        for t in [0.0, 0.5, 59.99, 1e6, -3.0, f32::NAN, f32::INFINITY] {
            let w = wave_normal(n, t, &params);
            assert!((w.length() - 1.0).abs() < 1e-4, "{n:?} t {t}");
            assert!(w.dot(n) > 0.9, "a gentle tilt");
        }
        // A huge time is the same as its place in the period.
        let wrapped = 1e6f32.rem_euclid(WAVE_PERIOD);
        assert!((wave_normal(n, 1e6, &params) - wave_normal(n, wrapped, &params)).length() < 1e-6);
    }
    // Foam only off the coast, inside the band.
    assert!(foam(-0.25 * FOAM_BAND, 1.0) > 0.0);
    assert_eq!(foam(0.01, 1.0), 0.0);
    assert_eq!(foam(-1.5 * FOAM_BAND, 1.0), 0.0);
    assert_eq!(foam(f32::NAN, 1.0), 0.0);
    assert!(foam(-0.5 * FOAM_BAND, 1e6).is_finite());
}

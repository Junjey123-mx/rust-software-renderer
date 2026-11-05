//! Deterministic lifecycles for repeating events.

use crate::shading::math::smoothstep;

/// Cycle index `k` and phase `tau` in `[0, 1)` of time `t` for a repeating
/// event of `period` seconds shifted by `phase` seconds. `(0, 0)` when the
/// period is not positive or the input is not finite.
pub fn cycle(t: f32, period: f32, phase: f32) -> (i64, f32) {
    if !(period > 0.0 && t.is_finite() && phase.is_finite()) {
        return (0, 0.0);
    }
    let x = (t + phase) / period;
    let k = x.floor();
    (k as i64, (x - k).clamp(0.0, 1.0))
}

/// Smooth rise, hold and fall over the phase; 0 outside and exactly 0 at
/// `tau = 0` and `tau = 1`, so an event can relocate between cycles without
/// a visible jump.
pub fn envelope(tau: f32, rise: f32, hold: f32, fall: f32) -> f32 {
    if !(tau > 0.0 && tau < 1.0) {
        return 0.0;
    }
    let up = smoothstep(0.0, rise, tau);
    let start_fall = rise + hold;
    let down = 1.0 - smoothstep(start_fall, (start_fall + fall).min(1.0), tau);
    up.min(down)
}

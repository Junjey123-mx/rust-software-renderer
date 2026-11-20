//! Hearts planet relief: a seeded fbm cut into terraces, so wide flat
//! plateaus (where the city stands) are split by stepped canyons. Each step
//! ends in a short smooth ramp, so the normals stay finite. Heights are in
//! planet radii.

use crate::shading::math::smoothstep;
use crate::shading::noise::fbm;
use crate::solar::planet::field::tangent;
use raylib::math::Vector3;

/// Height of the highest terrace, radii.
pub const HEARTS_HEIGHT: f32 = 0.06;
/// Share of each step taken by the ramp up to the next one.
const EDGE: f32 = 0.18;

/// Height field of the Hearts planet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeartsField {
    /// Terrace levels between the canyon floor and the top (at least 1).
    pub steps: f32,
    /// Height of the highest terrace, radii.
    pub height: f32,
    /// Frequency of the noise on the unit sphere.
    pub freq: f32,
    pub seed: u32,
    /// Largest slope (radii of height per radian) still read as plateau.
    pub plateau_slope: f32,
}

impl HeartsField {
    /// Four terraces up to `HEARTS_HEIGHT` for `seed`.
    pub fn new(seed: u32) -> Self {
        Self {
            steps: 4.0,
            height: HEARTS_HEIGHT,
            freq: 1.6,
            seed,
            plateau_slope: 0.05,
        }
    }

    /// Checks a field built by hand: finite values, at least one step, a
    /// height that is not negative, positive frequency and plateau slope.
    pub fn validate(&self) -> Result<(), String> {
        let values = [self.steps, self.height, self.freq, self.plateau_slope];
        if !values.iter().all(|v| v.is_finite()) {
            return Err("hearts field values must be finite".into());
        }
        if self.steps < 1.0 {
            return Err(format!(
                "hearts steps must be at least 1, got {}",
                self.steps
            ));
        }
        if self.height < 0.0 || self.freq <= 0.0 || self.plateau_slope <= 0.0 {
            return Err("hearts height must be >= 0, freq and plateau_slope > 0".into());
        }
        Ok(())
    }

    /// Terrace height above the radius along the unit `n`: the noise in
    /// `[0, 1]`, floored to `steps` levels with a smooth ramp at the end of
    /// each step.
    pub fn height(&self, n: Vector3) -> f32 {
        let steps = self.steps.max(1.0);
        let x = (0.5 + 0.75 * fbm(n * self.freq, 4, self.seed)).clamp(0.0, 1.0);
        let s = x * steps;
        let floor = s.floor();
        let ramp = smoothstep(1.0 - EDGE, 1.0, s - floor);
        ((floor + ramp) / steps).min(1.0) * self.height
    }

    /// Steepness at the unit `n`: the height gradient's length, radii per
    /// radian (central differences 1e-3 rad wide).
    pub fn slope(&self, n: Vector3) -> f32 {
        const EPS: f32 = 1e-3;
        let t1 = tangent(n);
        let t2 = n.cross(t1);
        let at = |d: Vector3| self.height((n + d * EPS).normalized());
        let du = (at(t1) - at(-t1)) / (2.0 * EPS);
        let dv = (at(t2) - at(-t2)) / (2.0 * EPS);
        du.hypot(dv)
    }

    /// Whether `n` lies on a flat terrace (the city's ground).
    pub fn is_plateau(&self, n: Vector3) -> bool {
        self.slope(n) < self.plateau_slope
    }

    /// Bound of `height`, radii.
    pub fn max_height(&self) -> f32 {
        self.height.max(0.0)
    }

    /// Digest of every parameter, for geometry keys.
    pub fn digest(&self) -> u64 {
        [
            self.steps.to_bits(),
            self.height.to_bits(),
            self.freq.to_bits(),
            self.plateau_slope.to_bits(),
            self.seed,
        ]
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}

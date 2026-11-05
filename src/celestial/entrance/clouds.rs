//! Crimson Entrance cloud layer.
//!
//! A mask over the same sphere (no extra geometry): fBm of a domain-warped
//! direction that rotates around the axis and evolves slowly, thresholded
//! by the coverage. The terrain never moves with it. Shadows sample the
//! same mask where the ray towards the light leaves a cloud shell.

use crate::celestial::entrance::EntranceParams;
use crate::shading::lighting::shell_exit;
use crate::shading::math::smoothstep;
use crate::shading::noise::{fbm, warp};
use crate::shading::sphere::rotate_y;
use raylib::math::Vector3;

/// Spreads the normalised fBm over the coverage threshold.
const CLOUD_CONTRAST: f32 = 1.6;

/// Cloud layer of one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudLayer {
    /// Rotation of the layer around the axis, radians.
    pub angle: f32,
    /// Slow evolution offset of the noise domain.
    pub drift: f32,
    pub coverage: f32,
    pub scale: f32,
    pub softness: f32,
    pub opacity: f32,
    pub enabled: bool,
    pub shadows: bool,
    pub shadow_strength: f32,
    /// Height of the cloud shell above the unit sphere.
    pub altitude: f32,
    seed: u32,
}

impl CloudLayer {
    pub fn new(p: &EntranceParams, seed: u32, t: f32) -> Self {
        Self {
            angle: p.cloud_speed * t,
            drift: p.cloud_evolution * t,
            coverage: p.cloud_coverage,
            scale: p.cloud_scale,
            softness: p.cloud_softness,
            opacity: p.cloud_opacity,
            enabled: p.clouds >= 0.5,
            shadows: p.cloud_shadows >= 0.5,
            shadow_strength: p.shadow_strength,
            altitude: p.cloud_altitude,
            seed,
        }
    }

    /// Cloud density in `[0, 1]` at unit direction `n`.
    pub fn mask(&self, n: Vector3) -> f32 {
        if !self.enabled || self.coverage <= 0.0 {
            return 0.0;
        }
        let q = rotate_y(n, -self.angle) * self.scale;
        let w = warp(q, 0.4, self.seed.wrapping_add(20)) + Vector3::new(0.0, self.drift, 0.0);
        let f = 0.5 + 0.5 * CLOUD_CONTRAST * fbm(w, 5, self.seed.wrapping_add(21));
        let threshold = 1.0 - self.coverage;
        smoothstep(threshold - self.softness, threshold + self.softness, f)
    }

    /// Direct-light factor in `[0, 1]` at surface direction `n` for unit
    /// light `l`: the cloud mask where the ray towards the light leaves the
    /// cloud shell. 1 on the night side and without shadows.
    pub fn shadow(&self, n: Vector3, l: Vector3) -> f32 {
        let mu = n.dot(l);
        if !self.shadows || mu <= 0.0 {
            return 1.0;
        }
        let q = shell_exit(n, l, self.altitude);
        1.0 - self.shadow_strength * self.mask(q) * smoothstep(0.0, 0.1, mu)
    }

    /// Opacity actually composited over the ground.
    pub fn alpha(&self, n: Vector3) -> f32 {
        self.mask(n) * self.opacity
    }
}

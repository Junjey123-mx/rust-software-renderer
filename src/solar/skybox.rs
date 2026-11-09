//! Procedural star field: unit directions generated once from a seed and
//! projected with the camera rotation only, so translating the camera never
//! moves the background.

use raylib::math::Vector3;

/// Most stars a field may hold.
pub const MAX_STARS: usize = 20_000;
/// Default star count and seed of the solar scene.
pub const DEFAULT_STARS: usize = 2_000;
pub const DEFAULT_SEED: u32 = 42;

/// Seeded linear congruential generator from the course guide: values in
/// `[0, 1)`; the same seed repeats the sequence.
pub fn random01(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 8) as f32) / 16_777_216.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    /// Unit direction, world space.
    pub direction: Vector3,
    /// In `[0.3, 1]`: most stars are faint and a few very bright.
    pub brightness: f32,
    /// Fixed at generation: between warm white and bluish white.
    pub tint: Vector3,
}

impl Star {
    pub fn color(&self) -> Vector3 {
        self.tint * self.brightness
    }
}

const WARM: Vector3 = Vector3 {
    x: 1.0,
    y: 0.92,
    z: 0.82,
};
const COOL: Vector3 = Vector3 {
    x: 0.80,
    y: 0.88,
    z: 1.0,
};

/// Uniform direction on the sphere: height uniform in `[-1, 1]` and an
/// angle around `Y`, so the poles are not denser.
pub fn random_direction(state: &mut u32) -> Vector3 {
    let y = 2.0 * random01(state) - 1.0;
    let angle = std::f32::consts::TAU * random01(state);
    let ring = (1.0 - y * y).max(0.0).sqrt();
    Vector3::new(ring * angle.cos(), y, ring * angle.sin())
}

#[derive(Debug, Clone, PartialEq)]
pub struct StarField {
    pub seed: u32,
    pub stars: Vec<Star>,
}

impl StarField {
    /// `count` stars (clamped to `MAX_STARS`) from `seed`.
    pub fn generate(seed: u32, count: usize) -> Self {
        let mut state = seed;
        let stars = (0..count.min(MAX_STARS))
            .map(|_| {
                let direction = random_direction(&mut state);
                let brightness = 0.3 + 0.7 * random01(&mut state).powi(3);
                let tint = WARM + (COOL - WARM) * random01(&mut state);
                Star {
                    direction,
                    brightness,
                    tint,
                }
            })
            .collect();
        Self { seed, stars }
    }
}

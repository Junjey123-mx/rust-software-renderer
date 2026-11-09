//! Procedural star field: unit directions generated once from a seed and
//! projected with the camera rotation only, so translating the camera never
//! moves the background.

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
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

/// Projects every star with the camera rotation (never its position) and
/// the perspective focal length, discarding stars behind the camera or off
/// screen, and writes color only: the depth buffer keeps its cleared value
/// so any body or the spacecraft drawn afterwards hides the background.
/// Returns the number of stars written.
pub fn draw_stars(
    framebuffer: &mut Framebuffer,
    field: &StarField,
    camera: &PerspectiveCamera,
) -> usize {
    let (w, h) = (framebuffer.width(), framebuffer.height());
    let basis = camera.basis();
    let focal = camera.focal_px(h);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let mut written = 0;
    for star in &field.stars {
        let d = star.direction;
        let z = d.dot(basis.forward);
        if z <= 0.0 {
            continue;
        }
        let x = (cx + focal * d.dot(basis.right) / z).floor();
        let y = (cy - focal * d.dot(basis.up) / z).floor();
        if !(x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32) {
            continue;
        }
        framebuffer.point(x as i32, y as i32, star.color());
        written += 1;
    }
    written
}

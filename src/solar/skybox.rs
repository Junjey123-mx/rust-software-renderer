//! Procedural star field: unit directions generated once from a seed and
//! projected with the camera rotation only, so translating the camera never
//! moves the background.

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
use crate::shading::noise::fbm;
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

/// How star positions are distributed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StarDensity {
    /// Uniform over the sphere.
    #[default]
    Uniform,
    /// Kept with a probability set by 3D noise on the direction: dense and
    /// sparse regions without angular seams.
    Clustered,
}

impl std::str::FromStr for StarDensity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uniform" => Ok(Self::Uniform),
            "clustered" => Ok(Self::Clustered),
            other => Err(format!(
                "unknown star density '{other}' (expected uniform or clustered)"
            )),
        }
    }
}

/// Noise frequency over the unit sphere: low values give broad regions.
pub const DENSITY_FREQUENCY: f32 = 2.5;
/// Lowest keep probability, so sparse regions never become empty.
pub const MIN_DENSITY: f32 = 0.12;

/// Keep probability of a candidate direction in `[MIN_DENSITY, 1]`. The
/// noise is evaluated once per candidate at generation, never per pixel.
pub fn density_at(direction: Vector3, noise_seed: u32) -> f32 {
    let n = fbm(direction * DENSITY_FREQUENCY, 3, noise_seed);
    (0.5 + 0.9 * n).clamp(MIN_DENSITY, 1.0)
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

    /// `attempts` candidates (clamped to `MAX_STARS`) distributed by
    /// `density`; clustered fields keep fewer stars than attempts. The noise
    /// seed derives from `seed`, so one seed fixes the whole sky.
    pub fn generate_with(seed: u32, attempts: usize, density: StarDensity) -> Self {
        if density == StarDensity::Uniform {
            return Self::generate(seed, attempts);
        }
        let mut state = seed;
        let noise_seed = seed ^ 0x5bd1_e995;
        let mut stars = Vec::new();
        for _ in 0..attempts.min(MAX_STARS) {
            let direction = random_direction(&mut state);
            let keep = random01(&mut state) < density_at(direction, noise_seed);
            let brightness = 0.3 + 0.7 * random01(&mut state).powi(3);
            let tint = WARM + (COOL - WARM) * random01(&mut state);
            if keep {
                stars.push(Star {
                    direction,
                    brightness,
                    tint,
                });
            }
        }
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

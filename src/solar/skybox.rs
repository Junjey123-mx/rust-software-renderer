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
    /// Fixed at generation: one of the stylized spectral classes.
    pub tint: Vector3,
}

impl Star {
    pub fn color(&self) -> Vector3 {
        self.tint * self.brightness
    }

    /// Footprint on screen, fixed by the brightness.
    pub fn size(&self) -> StarSize {
        if self.brightness > GLINT_STAR {
            StarSize::Glint
        } else if self.brightness > BRIGHT_STAR {
            StarSize::Bright
        } else {
            StarSize::Dot
        }
    }
}

/// Stylized spectral classes with the cumulative share of stars up to each:
/// blue-white, white, yellow, orange and red.
pub const SPECTRAL: [(Vector3, f32); 5] = [
    (Vector3::new(0.72, 0.82, 1.0), 0.18),
    (Vector3::new(1.0, 0.97, 0.94), 0.45),
    (Vector3::new(1.0, 0.88, 0.60), 0.67),
    (Vector3::new(1.0, 0.68, 0.40), 0.85),
    (Vector3::new(1.0, 0.46, 0.38), 1.0),
];

/// Tint of the spectral class `u` in `[0, 1)` falls in.
pub fn spectral_tint(u: f32) -> Vector3 {
    SPECTRAL
        .iter()
        .find(|(_, share)| u < *share)
        .map_or(SPECTRAL[4].0, |(tint, _)| *tint)
}

/// Brightness above which a star is drawn as a plus (about 4 % of them)
/// and as a glint (well under 1 %).
pub const BRIGHT_STAR: f32 = 0.92;
pub const GLINT_STAR: f32 = 0.985;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarSize {
    /// One pixel: almost every star.
    Dot,
    /// A soft five-pixel plus.
    Bright,
    /// A plus whose arms fade over three pixels: an intense distant star.
    Glint,
}

impl StarSize {
    /// Pixels as `(dx, dy, weight)`, the center first.
    pub fn footprint(self) -> &'static [(i32, i32, f32)] {
        match self {
            Self::Dot => &[(0, 0, 1.0)],
            Self::Bright => &[
                (0, 0, 1.0),
                (1, 0, 0.45),
                (-1, 0, 0.45),
                (0, 1, 0.45),
                (0, -1, 0.45),
            ],
            Self::Glint => &[
                (0, 0, 1.0),
                (1, 0, 0.6),
                (-1, 0, 0.6),
                (0, 1, 0.6),
                (0, -1, 0.6),
                (1, 1, 0.2),
                (-1, 1, 0.2),
                (1, -1, 0.2),
                (-1, -1, 0.2),
                (2, 0, 0.3),
                (-2, 0, 0.3),
                (0, 2, 0.3),
                (0, -2, 0.3),
                (3, 0, 0.12),
                (-3, 0, 0.12),
                (0, 3, 0.12),
                (0, -3, 0.12),
            ],
        }
    }
}

/// Most pixels one star may touch.
pub const MAX_FOOTPRINT: usize = 17;

/// The brightness and spectral class drawn after a star's direction.
fn star_at(direction: Vector3, state: &mut u32) -> Star {
    let brightness = 0.3 + 0.7 * random01(state).powi(3);
    Star {
        direction,
        brightness,
        tint: spectral_tint(random01(state)),
    }
}

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
    /// A few faint comet streaks; none when the field has no stars.
    pub streaks: Vec<Streak>,
}

/// Comet streaks over the whole sphere: about one in view at a time.
pub const STREAKS: usize = 12;
/// Angular length of a streak, radians (4 to 9 degrees).
pub const STREAK_LENGTH: (f32, f32) = (0.07, 0.157);
/// Pale blue-white of the streaks, scaled by a brightness in `[0.45, 0.75]`.
const STREAK_TINT: Vector3 = Vector3::new(0.80, 0.88, 1.0);

/// A static comet trace on the sky: brightest at `head`, fading to nothing
/// at `tail` (both unit directions).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Streak {
    pub head: Vector3,
    pub tail: Vector3,
    pub color: Vector3,
}

/// `STREAKS` streaks from their own sequence of `seed`, so adding them
/// leaves every star of the seed where it was.
pub fn comet_streaks(seed: u32) -> Vec<Streak> {
    let mut state = seed ^ 0x6c8e_9cf5;
    (0..STREAKS)
        .map(|_| {
            let head = random_direction(&mut state);
            let side = head.cross(random_direction(&mut state));
            let side = if side.length() > 1e-3 {
                side.normalized()
            } else {
                head.cross(Vector3::new(1.0, 0.0, 0.0)).normalized()
            };
            let (lo, hi) = STREAK_LENGTH;
            let length = lo + (hi - lo) * random01(&mut state);
            let tail = head * length.cos() + side * length.sin();
            let color = STREAK_TINT * (0.45 + 0.3 * random01(&mut state));
            Streak { head, tail, color }
        })
        .collect()
}

fn with_streaks(seed: u32, stars: Vec<Star>) -> StarField {
    let streaks = if stars.is_empty() {
        Vec::new()
    } else {
        comet_streaks(seed)
    };
    StarField {
        seed,
        stars,
        streaks,
    }
}

impl StarField {
    /// `count` stars (clamped to `MAX_STARS`) from `seed`.
    pub fn generate(seed: u32, count: usize) -> Self {
        let mut state = seed;
        let stars = (0..count.min(MAX_STARS))
            .map(|_| {
                let direction = random_direction(&mut state);
                star_at(direction, &mut state)
            })
            .collect();
        with_streaks(seed, stars)
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
            let star = star_at(direction, &mut state);
            if keep {
                stars.push(star);
            }
        }
        with_streaks(seed, stars)
    }
}

/// Projects every star with the camera rotation (never its position) and
/// the perspective focal length, discarding stars behind the camera or off
/// screen, and adds its footprint to the color already there (clamped to
/// 1). The comet streaks are drawn first, the same way. Color only: the depth buffer keeps its cleared value so any body or
/// the spacecraft drawn afterwards hides the background. Returns the number
/// of stars written.
pub fn draw_stars(
    framebuffer: &mut Framebuffer,
    field: &StarField,
    camera: &PerspectiveCamera,
) -> usize {
    let (w, h) = (framebuffer.width(), framebuffer.height());
    let basis = camera.basis();
    let focal = camera.focal_px(h);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    // Screen position of a direction in front of the camera.
    let project = |d: Vector3| {
        let z = d.dot(basis.forward);
        (z > 0.0).then(|| {
            (
                (cx + focal * d.dot(basis.right) / z).floor(),
                (cy - focal * d.dot(basis.up) / z).floor(),
            )
        })
    };
    // Within one screen of the image: a streak with an end farther out
    // (nearly sideways to the view) would cost a long off-screen walk.
    let (wf, hf) = (w as f32, h as f32);
    let near = |(x, y): (f32, f32)| x > -wf && x < 2.0 * wf && y > -hf && y < 2.0 * hf;
    for streak in &field.streaks {
        if let (Some(head), Some(tail)) = (project(streak.head), project(streak.tail))
            && near(head)
            && near(tail)
        {
            draw_streak(framebuffer, head, tail, streak.color);
        }
    }
    let mut written = 0;
    for star in &field.stars {
        let Some((x, y)) = project(star.direction) else {
            continue;
        };
        if !(x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32) {
            continue;
        }
        let color = star.color();
        for &(dx, dy, weight) in star.size().footprint() {
            add_color(framebuffer, x as i32 + dx, y as i32 + dy, color * weight);
        }
        written += 1;
    }
    written
}

/// A line from `head` to `tail` whose light fades quadratically to zero,
/// added to the pixels it crosses (off-screen ones are skipped).
fn draw_streak(framebuffer: &mut Framebuffer, head: (f32, f32), tail: (f32, f32), color: Vector3) {
    let (dx, dy) = (tail.0 - head.0, tail.1 - head.1);
    let steps = dx.abs().max(dy.abs()).max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let fade = (1.0 - t) * (1.0 - t);
        let (x, y) = (head.0 + dx * t, head.1 + dy * t);
        add_color(framebuffer, x as i32, y as i32, color * fade);
    }
}

/// Adds `color` to the pixel, clamped to 1; off-screen pixels are skipped.
fn add_color(framebuffer: &mut Framebuffer, x: i32, y: i32, color: Vector3) {
    if let Some(p) = framebuffer.get_pixel(x, y) {
        let sum = p + color;
        framebuffer.point(
            x,
            y,
            Vector3::new(sum.x.min(1.0), sum.y.min(1.0), sum.z.min(1.0)),
        );
    }
}

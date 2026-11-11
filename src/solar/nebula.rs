//! Nebula backdrop of the solar sky in the Red-Black palette (black
//! violet, deep purple, wine, corinth and dark magenta). The colors are
//! painted once on a small longitude-latitude map from a seed; each frame
//! samples that map on a coarse screen grid with the camera rotation only
//! and interpolates between grid points, so the cost does not grow with
//! the noise and translating the camera never moves the background.

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
use crate::shading::noise::fbm;
use raylib::math::Vector3;
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

/// Size of the longitude-latitude map: about 1.4 degrees per texel.
pub const MAP_WIDTH: usize = 256;
pub const MAP_HEIGHT: usize = 128;
/// Spacing of the sampled screen grid, pixels.
pub const GRID_STEP: u32 = 8;

pub const VOID: Vector3 = Vector3::new(0.030, 0.008, 0.040);
pub const DEEP_PURPLE: Vector3 = Vector3::new(0.110, 0.030, 0.180);
pub const WINE: Vector3 = Vector3::new(0.160, 0.022, 0.065);
pub const CORINTH: Vector3 = Vector3::new(0.250, 0.040, 0.100);
pub const MAGENTA: Vector3 = Vector3::new(0.330, 0.050, 0.230);

/// Normal of the great circle the brightest band follows: tilted from the
/// orbital plane so the band crosses the view diagonally.
const BAND_NORMAL: Vector3 = Vector3::new(0.42, 0.85, 0.32);
/// Angular half-width of the band (sine of the angle to its circle).
const BAND_WIDTH: f32 = 0.32;

#[derive(Debug, Clone)]
pub struct Nebula {
    pub seed: u32,
    /// Scale of the painted color: 0 leaves the plain black sky, 1 is the
    /// designed look.
    pub intensity: f32,
    /// Painted on first use, so scenes that never draw the sky never pay.
    map: OnceLock<Vec<Vector3>>,
}

impl Nebula {
    pub fn new(seed: u32, intensity: f32) -> Self {
        Self {
            seed,
            intensity: intensity.clamp(0.0, 1.0),
            map: OnceLock::new(),
        }
    }

    /// Designed color along a unit direction (intensity 1): a vertical
    /// gradient from black violet above to wine below, and a band of purple
    /// and magenta clouds shaped by two layers of noise.
    pub fn color_at(&self, d: Vector3) -> Vector3 {
        let broad = fbm(d * 1.6, 4, self.seed ^ 0x2c1b_3c6d);
        let wisps = fbm(d * 4.2, 4, self.seed ^ 0x7f4a_7c15);
        let below = smoothstep(0.7, -0.8, d.y);
        let base = VOID + (WINE - VOID) * (0.6 * below);
        let offset = d.dot(BAND_NORMAL.normalized()) + 0.35 * broad;
        let band = (-(offset * offset) / (BAND_WIDTH * BAND_WIDTH)).exp();
        // The smoothstep opens dark gaps between the clouds.
        let clouds = smoothstep(
            0.08,
            0.9,
            band * (0.55 + 1.1 * broad) + 0.30 * (2.0 * wisps).max(0.0) * (0.3 + band),
        );
        let tint = DEEP_PURPLE + (MAGENTA - DEEP_PURPLE) * (0.5 + 1.4 * wisps).clamp(0.0, 1.0);
        let core = CORINTH * (clouds * clouds * band);
        base + tint * clouds * 0.75 + core * 0.6
    }

    fn map(&self) -> &[Vector3] {
        self.map.get_or_init(|| {
            let mut map = Vec::with_capacity(MAP_WIDTH * MAP_HEIGHT);
            for j in 0..MAP_HEIGHT {
                let theta = (j as f32 + 0.5) / MAP_HEIGHT as f32 * PI;
                for i in 0..MAP_WIDTH {
                    let phi = (i as f32 + 0.5) / MAP_WIDTH as f32 * TAU - PI;
                    let d = Vector3::new(
                        theta.sin() * phi.cos(),
                        theta.cos(),
                        theta.sin() * phi.sin(),
                    );
                    map.push(self.color_at(d));
                }
            }
            map
        })
    }

    /// Bilinear sample of the painted map along a unit direction, scaled
    /// by the intensity. Longitude wraps; latitude clamps at the poles.
    pub fn sample(&self, d: Vector3) -> Vector3 {
        let map = self.map();
        let u = (d.z.atan2(d.x) + PI) / TAU * MAP_WIDTH as f32 - 0.5;
        let v = d.y.clamp(-1.0, 1.0).acos() / PI * MAP_HEIGHT as f32 - 0.5;
        let (fu, fv) = (u - u.floor(), v - v.floor());
        let i0 = (u.floor() as i32).rem_euclid(MAP_WIDTH as i32) as usize;
        let i1 = (i0 + 1) % MAP_WIDTH;
        let row = |j: f32| (j.clamp(0.0, (MAP_HEIGHT - 1) as f32) as usize) * MAP_WIDTH;
        let (r0, r1) = (row(v.floor()), row(v.floor() + 1.0));
        let top = map[r0 + i0] + (map[r0 + i1] - map[r0 + i0]) * fu;
        let bottom = map[r1 + i0] + (map[r1 + i1] - map[r1 + i0]) * fu;
        (top + (bottom - top) * fv) * self.intensity
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Paints the backdrop over the whole frame: the map is sampled every
/// `GRID_STEP` pixels along the camera's view directions and interpolated
/// in between. Color only: the depth buffer keeps its cleared value, so
/// every body drawn afterwards hides the backdrop. Intensity 0 draws
/// nothing.
pub fn draw_nebula(framebuffer: &mut Framebuffer, nebula: &Nebula, camera: &PerspectiveCamera) {
    if nebula.intensity <= 0.0 {
        return;
    }
    let (w, h) = (framebuffer.width(), framebuffer.height());
    let basis = camera.basis();
    let focal = camera.focal_px(h);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let (cols, rows) = (w / GRID_STEP + 2, h / GRID_STEP + 2);
    let mut grid = Vec::with_capacity((cols * rows) as usize);
    for gy in 0..rows {
        let y = (gy * GRID_STEP) as f32 + 0.5;
        for gx in 0..cols {
            let x = (gx * GRID_STEP) as f32 + 0.5;
            let d = basis.forward * focal + basis.right * (x - cx) - basis.up * (y - cy);
            grid.push(nebula.sample(d.normalized()));
        }
    }
    // Bilinear in two passes: each pixel row first blends its two grid
    // rows once, then every pixel blends two neighbors of that line.
    let step = GRID_STEP as f32;
    let cols = cols as usize;
    let mut line = vec![Vector3::zero(); cols];
    for y in 0..h {
        let fy = (y % GRID_STEP) as f32 / step;
        let r0 = (y / GRID_STEP) as usize * cols;
        for (c, v) in line.iter_mut().enumerate() {
            *v = grid[r0 + c] + (grid[r0 + cols + c] - grid[r0 + c]) * fy;
        }
        let Some(row) = framebuffer.row_mut(y) else {
            break;
        };
        for (cell, pixels) in row.chunks_mut(GRID_STEP as usize).enumerate() {
            let (start, delta) = (line[cell], (line[cell + 1] - line[cell]) / step);
            for (k, pixel) in pixels.iter_mut().enumerate() {
                *pixel = start + delta * k as f32;
            }
        }
    }
}

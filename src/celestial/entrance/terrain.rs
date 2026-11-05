//! Crimson Entrance height field, regions and fissures.
//!
//! One mathematical field drives everything: a monumental basin plus fBm
//! (macro height, later used by parallax) and a finer fBm (detail height,
//! later used by bump mapping). Materials are thresholds of that field, so
//! relief, regions and lighting always agree.

use crate::celestial::entrance::Frame;
use crate::celestial::entrance::events::{Relief, event_relief};
use crate::shading::math::{hex, mix3, smoothstep};
use crate::shading::noise::{fbm, gradient3};
use crate::shading::sphere::angular_distance;
use raylib::math::Vector3;

/// Bowl of depth `depth` for normalised distance `d < 1` plus a raised rim
/// around `d = 1`.
pub fn crater_profile(d: f32, depth: f32) -> f32 {
    let bowl = if d < 1.0 { -depth * (1.0 - d * d) } else { 0.0 };
    bowl + 0.35 * depth * rim_profile(d)
}

/// Rim strength in `[0, 1]`, peaking at `d = 1`.
pub fn rim_profile(d: f32) -> f32 {
    (-((d - 1.0) / 0.18).powi(2)).exp()
}

fn basin_distance(n: Vector3, f: &Frame) -> f32 {
    angular_distance(n, f.basin_center) / f.params.basin_radius
}

/// Large-scale relief: the basin, low-frequency hills and, when `events`
/// is on, the migrating craters and spade mesas.
pub fn macro_height(n: Vector3, f: &Frame) -> f32 {
    macro_parts(n, f).0
}

fn macro_parts(n: Vector3, f: &Frame) -> (f32, Relief) {
    let p = &f.params;
    let relief = if p.events >= 0.5 {
        event_relief(n, f.events.as_slice(), f.seed)
    } else {
        Relief::default()
    };
    let h = crater_profile(basin_distance(n, f), p.basin_depth)
        + MACRO_AMPLITUDE * fbm(n * p.terrain_scale, 2, f.seed)
        + relief.height;
    (h, relief)
}

/// Amplitude of the low-frequency hills of the macro height.
const MACRO_AMPLITUDE: f32 = 0.6;

/// Full height: macro relief plus fine detail.
pub fn height(n: Vector3, f: &Frame) -> f32 {
    height_parts(n, f).0
}

fn height_parts(n: Vector3, f: &Frame) -> (f32, Relief) {
    let p = &f.params;
    let (macro_h, relief) = macro_parts(n, f);
    let h = macro_h
        + p.detail_amplitude
            * fbm(
                n * (3.0 * p.terrain_scale),
                p.terrain_octaves.round() as u32,
                f.seed.wrapping_add(5),
            );
    (h, relief)
}

/// Fissure network in `[0, 1]`: thin zero crossings of an fBm, kept to low
/// ground (`h < 0.1`).
pub fn crack_mask(n: Vector3, h: f32, f: &Frame) -> f32 {
    let p = &f.params;
    let line = fbm(n * p.crack_scale, 3, f.seed.wrapping_add(7)).abs();
    (1.0 - smoothstep(0.0, p.crack_width, line)) * (1.0 - smoothstep(0.05, 0.1, h))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainMaterial {
    /// Glassy plains on the low ground.
    Obsidian,
    Basalt,
    /// Crimson highlands.
    Highland,
    /// Slate mesas on the highest ground.
    Slate,
    /// Ash on crater and basin rims.
    Ash,
    /// Glowing fissures.
    Crack,
}

impl TerrainMaterial {
    pub const ALL: [TerrainMaterial; 6] = [
        Self::Obsidian,
        Self::Basalt,
        Self::Highland,
        Self::Slate,
        Self::Ash,
        Self::Crack,
    ];

    /// How the material reflects light: glassy obsidian, wrapped ash,
    /// self-lit fissures and matte rock.
    pub fn response(self) -> Response {
        match self {
            Self::Obsidian => Response {
                specular: 1.0,
                wrap: 0.0,
                emissive: false,
            },
            Self::Ash => Response {
                specular: 0.0,
                wrap: 0.3,
                emissive: false,
            },
            Self::Crack => Response {
                specular: 0.0,
                wrap: 0.0,
                emissive: true,
            },
            Self::Basalt | Self::Highland | Self::Slate => Response {
                specular: 0.0,
                wrap: 0.0,
                emissive: false,
            },
        }
    }

    /// Saturated flat color of the `materials` debug view.
    pub fn debug_color(self) -> Vector3 {
        match self {
            Self::Obsidian => hex(0x2040FF),
            Self::Basalt => hex(0x404040),
            Self::Highland => hex(0xFF2030),
            Self::Slate => hex(0x30D0C0),
            Self::Ash => hex(0xF0F0F0),
            Self::Crack => hex(0xFF9000),
        }
    }
}

/// Reflection model of a terrain material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Response {
    /// Weight of the Blinn-Phong highlight (0 = matte).
    pub specular: f32,
    /// Diffuse wrap past the terminator (dusty surfaces).
    pub wrap: f32,
    /// Lit from within: the color does not depend on the light.
    pub emissive: bool,
}

/// Region of a point, in priority order: fissure, rim ash, then height bands.
pub fn classify(h: f32, rim: f32, crack: f32) -> TerrainMaterial {
    if crack > 0.5 {
        TerrainMaterial::Crack
    } else if rim > 0.25 {
        TerrainMaterial::Ash
    } else if h < -0.12 {
        TerrainMaterial::Obsidian
    } else if h < 0.08 {
        TerrainMaterial::Basalt
    } else if h < 0.2 {
        TerrainMaterial::Highland
    } else {
        TerrainMaterial::Slate
    }
}

/// Base color of a material, varied between two tones by noise.
pub fn albedo(material: TerrainMaterial, n: Vector3, f: &Frame) -> Vector3 {
    let t = 0.5 + 0.5 * gradient3(n * 11.0, f.seed.wrapping_add(9));
    let (a, b) = match material {
        TerrainMaterial::Obsidian => (hex(0x4A0B10), hex(0x8F1018)),
        TerrainMaterial::Basalt => (hex(0x151515), hex(0x252525)),
        TerrainMaterial::Highland => (hex(0x8F1018), hex(0xD21F2B)),
        TerrainMaterial::Slate => (hex(0x3A3A42), hex(0x5A1A1E)),
        TerrainMaterial::Ash => (hex(0xD6D0C4) * 0.8, hex(0xD6D0C4) * 0.65),
        TerrainMaterial::Crack => (hex(0xFF7A00), hex(0xFF7A00)),
    };
    mix3(a, b, t)
}

/// Everything the shader needs about the ground at one direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    pub height: f32,
    pub rim: f32,
    pub crack: f32,
    /// Molten crater floor glow in `[0, 1]`.
    pub magma: f32,
    pub material: TerrainMaterial,
}

pub fn ground(n: Vector3, f: &Frame) -> Ground {
    let (h, relief) = height_parts(n, f);
    // Irregular dusty edge: a narrow rim band broken up by noise.
    let d = basin_distance(n, f);
    let rim = (-((d - 1.0) / 0.1).powi(2)).exp()
        * (0.5 + 1.4 * fbm(n * 5.0, 2, f.seed.wrapping_add(11))).clamp(0.0, 1.0);
    let rim = rim.max(relief.rim * (0.6 + 0.4 * gradient3(n * 13.0, f.seed.wrapping_add(12))));
    let crack = crack_mask(n, h, f).max(relief.fracture);
    Ground {
        height: h,
        rim,
        crack,
        magma: relief.magma,
        material: classify(h, rim, crack.max(relief.magma)),
    }
}

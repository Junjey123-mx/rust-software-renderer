//! Crimson Entrance height field, regions and fissures.
//!
//! One mathematical field drives everything: a monumental basin plus fBm
//! (macro height, later used by parallax) and a finer fBm (detail height,
//! later used by bump mapping). Materials are thresholds of that field, so
//! relief, regions and lighting always agree.
//!
//! Visual hierarchy: black and gray volcanic rock (obsidian, basalt, slate)
//! is the canvas; matte crimson highlands and ash rims come next, with an
//! ochre and brown crust where materials meet; thin amber fissures follow;
//! the hyperincandescent lava is reserved for the floor of spade craters.

use crate::celestial::entrance::Frame;
use crate::celestial::entrance::events::CraterSurface;
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
/// is on, the spade craters (height only: no masks, no glow).
pub fn macro_height(n: Vector3, f: &Frame) -> f32 {
    let events: f32 = if f.params.events >= 0.5 {
        f.events.iter().map(|e| e.height(n)).sum()
    } else {
        0.0
    };
    base_height(n, f) + events
}

/// The basin and the hills.
fn base_height(n: Vector3, f: &Frame) -> f32 {
    let p = &f.params;
    crater_profile(basin_distance(n, f), p.basin_depth)
        + MACRO_AMPLITUDE * fbm(n * p.terrain_scale, 2, f.seed)
}

/// Fine relief added on top of the macro height.
fn detail_height(n: Vector3, f: &Frame) -> f32 {
    let p = &f.params;
    p.detail_amplitude
        * fbm(
            n * (3.0 * p.terrain_scale),
            p.terrain_octaves.round() as u32,
            f.seed.wrapping_add(5),
        )
}

/// Amplitude of the low-frequency hills of the macro height.
const MACRO_AMPLITUDE: f32 = 0.6;

/// Full height: macro relief plus fine detail.
pub fn height(n: Vector3, f: &Frame) -> f32 {
    macro_height(n, f) + detail_height(n, f)
}

/// Fissure network in `[0, 1]`: thin zero crossings of an fBm, kept to the
/// lower ground (`h < 0.02`) so it stays below the craters in importance.
pub fn crack_mask(n: Vector3, h: f32, f: &Frame) -> f32 {
    let p = &f.params;
    let line = fbm(n * p.crack_scale, 3, f.seed.wrapping_add(7)).abs();
    (1.0 - smoothstep(0.0, p.crack_width, line)) * (1.0 - smoothstep(-0.04, 0.02, h))
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
    /// Molten floor of a spade crater.
    Lava,
}

impl TerrainMaterial {
    pub const ALL: [TerrainMaterial; 7] = [
        Self::Obsidian,
        Self::Basalt,
        Self::Highland,
        Self::Slate,
        Self::Ash,
        Self::Crack,
        Self::Lava,
    ];

    /// How the material reflects light: glassy obsidian, wrapped ash,
    /// self-lit fissures and lava, and matte rock.
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
            Self::Crack | Self::Lava => Response {
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
            Self::Lava => hex(0xFF00C8),
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

/// Height where basalt gives way to the crimson highlands.
pub const HIGHLAND_FROM: f32 = 0.08;

/// Region of a point, in priority order: molten crater floor, fissure, rim
/// ash, then height bands.
pub fn classify(h: f32, rim: f32, crack: f32, lava: f32) -> TerrainMaterial {
    if lava > 0.3 {
        TerrainMaterial::Lava
    } else if crack > 0.5 {
        TerrainMaterial::Crack
    } else if rim > 0.25 {
        TerrainMaterial::Ash
    } else if h < -0.12 {
        TerrainMaterial::Obsidian
    } else if h < HIGHLAND_FROM {
        TerrainMaterial::Basalt
    } else if h < 0.2 {
        TerrainMaterial::Highland
    } else {
        TerrainMaterial::Slate
    }
}

/// Base color of a material, varied between two tones by noise: black and
/// gray rock, matte wine to blood red highlands (never lava bright), ash
/// grays, amber fissures and red-hot lava.
pub fn albedo(material: TerrainMaterial, n: Vector3, f: &Frame) -> Vector3 {
    let t = 0.5 + 0.5 * gradient3(n * 11.0, f.seed.wrapping_add(9));
    let (a, b) = match material {
        TerrainMaterial::Obsidian => (hex(0x151515), hex(0x252525)),
        TerrainMaterial::Basalt => (hex(0x252525), hex(0x3A3A42)),
        TerrainMaterial::Highland => (hex(0x4A0B10), hex(0x8F1018)),
        TerrainMaterial::Slate => (hex(0x3A3A42), hex(0x4A4850)),
        TerrainMaterial::Ash => (hex(0x6E6468), hex(0x858080)),
        TerrainMaterial::Crack => (hex(0xFF7A00), hex(0xFF9A28)),
        TerrainMaterial::Lava => (hex(0xFF2020), hex(0xFF4040)),
    };
    mix3(a, b, t)
}

/// Everything the shader needs about the ground at one direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    pub height: f32,
    pub rim: f32,
    pub crack: f32,
    /// Fractures of a spade crater (self-lit) in `[0, 1]`.
    pub fracture: f32,
    /// Ochre and brown crust where materials meet and around open craters.
    pub crust: f32,
    /// Molten floor of a spade crater in `[0, 1]`.
    pub lava: f32,
    /// White-hot core of the floor in `[0, 1]`.
    pub core: f32,
    /// Carbonized rock of a spade crater in `[0, 1]`.
    pub char: f32,
    pub material: TerrainMaterial,
}

pub fn ground(n: Vector3, f: &Frame) -> Ground {
    // One pass over the spade craters gives their height and every mask;
    // the sum is the same `macro_height + detail` the lookups use.
    let mut event = CraterSurface::default();
    if f.params.events >= 0.5 {
        for e in f.events.iter() {
            let s = e.surface(n);
            event.height += s.height;
            event.rim = event.rim.max(s.rim);
            event.fracture = event.fracture.max(s.fracture);
            event.lava = event.lava.max(s.lava);
            event.core = event.core.max(s.core);
            event.char = event.char.max(s.char);
            event.crust = event.crust.max(s.crust);
        }
    }
    let h = base_height(n, f) + event.height + detail_height(n, f);
    // Irregular dusty edge: a narrow rim band broken up by noise.
    let d = basin_distance(n, f);
    let rim = (-((d - 1.0) / 0.1).powi(2)).exp()
        * (0.5 + 1.4 * fbm(n * 5.0, 2, f.seed.wrapping_add(11))).clamp(0.0, 1.0);
    let grain = gradient3(n * 13.0, f.seed.wrapping_add(12));
    let rim = rim.max(event.rim * (0.6 + 0.4 * grain));
    let crack = crack_mask(n, h, f).max(event.fracture);
    // Dry crust in a broken band where basalt meets the highlands.
    let seam = 1.0 - smoothstep(0.0, 0.035, (h - HIGHLAND_FROM).abs());
    Ground {
        height: h,
        rim,
        crack,
        fracture: event.fracture,
        crust: (seam * (0.55 + 0.45 * grain)).max(event.crust),
        lava: event.lava,
        core: event.core,
        char: event.char,
        material: classify(h, rim, crack, event.lava),
    }
}

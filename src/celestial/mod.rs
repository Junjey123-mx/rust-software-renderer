//! Laboratory 6 celestial bodies on the course sphere.
//!
//! Every body is the same expanded `sphere.obj`; only its fragment shader
//! differs. A body has a table of named parameters (`ParamSpec`), a material
//! that validates them and a `PreparedShader` that caches per-frame values
//! (event centers, frames, rotations) before shading the fragments.

use crate::pipeline::{ShaderUniforms, SurfaceSample};
use crate::shading::math::hex;
use raylib::math::Vector3;
use std::fmt;
use std::str::FromStr;

/// One adjustable shader parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    pub name: &'static str,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub help: &'static str,
}

/// Declares a parameter struct with its `SPECS` table, `Default`, `get` and
/// a validated `set`, so the table and the fields cannot drift apart.
macro_rules! params {
    ($ty:ident { $($field:ident : $default:expr, $min:expr, $max:expr, $help:expr;)* }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $ty {
            $(pub $field: f32,)*
        }

        impl Default for $ty {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }

        impl $ty {
            pub const SPECS: &'static [$crate::celestial::ParamSpec] = &[
                $($crate::celestial::ParamSpec {
                    name: stringify!($field),
                    default: $default,
                    min: $min,
                    max: $max,
                    help: $help,
                },)*
            ];

            pub fn get(&self, name: &str) -> Option<f32> {
                match name {
                    $(stringify!($field) => Some(self.$field),)*
                    _ => None,
                }
            }

            pub fn set(&mut self, name: &str, value: f32) -> Result<(), String> {
                let spec = $crate::celestial::find_spec(Self::SPECS, name)?;
                $crate::celestial::check_range(spec, value)?;
                match name {
                    $(stringify!($field) => self.$field = value,)*
                    _ => unreachable!("spec table and fields match"),
                }
                Ok(())
            }
        }
    };
}

pub mod cli;
pub mod crown;
pub mod entrance;
pub mod house;
pub mod scene;
pub mod viewer;

pub use scene::CelestialScene;

pub(crate) fn find_spec(
    specs: &'static [ParamSpec],
    name: &str,
) -> Result<&'static ParamSpec, String> {
    specs.iter().find(|s| s.name == name).ok_or_else(|| {
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        format!(
            "unknown parameter '{name}' (expected one of: {})",
            names.join(", ")
        )
    })
}

pub(crate) fn check_range(spec: &ParamSpec, value: f32) -> Result<(), String> {
    if value.is_finite() && value >= spec.min && value <= spec.max {
        Ok(())
    } else {
        Err(format!(
            "parameter '{}' = {value} is outside [{}, {}]",
            spec.name, spec.min, spec.max
        ))
    }
}

/// The three bodies of laboratory 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Body {
    #[default]
    KingsCrown,
    CrimsonEntrance,
    HouseOfCards,
}

impl Body {
    pub const ALL: [Body; 3] = [Self::KingsCrown, Self::CrimsonEntrance, Self::HouseOfCards];

    /// Command-line identifier.
    pub fn name(self) -> &'static str {
        match self {
            Self::KingsCrown => "crown",
            Self::CrimsonEntrance => "entrance",
            Self::HouseOfCards => "house",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::KingsCrown => "The King's Crown",
            Self::CrimsonEntrance => "Crimson Entrance",
            Self::HouseOfCards => "House of Cards",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::KingsCrown => Self::CrimsonEntrance,
            Self::CrimsonEntrance => Self::HouseOfCards,
            Self::HouseOfCards => Self::KingsCrown,
        }
    }

    /// Accent color, used for the wireframe view of the shared sphere.
    pub fn accent(self) -> Vector3 {
        match self {
            Self::KingsCrown => hex(0xFFC63A),
            Self::CrimsonEntrance => hex(0xD21F2B),
            Self::HouseOfCards => hex(0xC13CFF),
        }
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Body {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|b| b.name() == s)
            .ok_or_else(|| format!("unknown body '{s}' (expected crown, entrance or house)"))
    }
}

/// Diagnostic views of the shader inputs (`--debug-view`, key `N`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugView {
    #[default]
    None,
    Height,
    Materials,
    Clouds,
    Shadows,
    Normals,
    Night,
    Parallax,
}

impl DebugView {
    pub const ALL: [DebugView; 8] = [
        Self::None,
        Self::Height,
        Self::Materials,
        Self::Clouds,
        Self::Shadows,
        Self::Normals,
        Self::Night,
        Self::Parallax,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Height => "height",
            Self::Materials => "materials",
            Self::Clouds => "clouds",
            Self::Shadows => "shadows",
            Self::Normals => "normals",
            Self::Night => "night",
            Self::Parallax => "parallax",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

impl FromStr for DebugView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|v| v.name() == s).ok_or_else(|| {
            format!(
                "unknown debug view '{s}' (expected none, height, materials, clouds, shadows, normals, night or parallax)"
            )
        })
    }
}

/// Validated parameters of one body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CelestialMaterial {
    Crown(crown::CrownParams),
    Entrance(entrance::EntranceParams),
    House(house::HouseParams),
}

impl CelestialMaterial {
    pub fn for_body(body: Body) -> Self {
        match body {
            Body::KingsCrown => Self::Crown(Default::default()),
            Body::CrimsonEntrance => Self::Entrance(Default::default()),
            Body::HouseOfCards => Self::House(Default::default()),
        }
    }

    pub fn body(&self) -> Body {
        match self {
            Self::Crown(_) => Body::KingsCrown,
            Self::Entrance(_) => Body::CrimsonEntrance,
            Self::House(_) => Body::HouseOfCards,
        }
    }

    pub fn specs(&self) -> &'static [ParamSpec] {
        match self {
            Self::Crown(_) => crown::CrownParams::SPECS,
            Self::Entrance(_) => entrance::EntranceParams::SPECS,
            Self::House(_) => house::HouseParams::SPECS,
        }
    }

    pub fn get(&self, name: &str) -> Option<f32> {
        match self {
            Self::Crown(p) => p.get(name),
            Self::Entrance(p) => p.get(name),
            Self::House(p) => p.get(name),
        }
    }

    pub fn set(&mut self, name: &str, value: f32) -> Result<(), String> {
        match self {
            Self::Crown(p) => p.set(name, value),
            Self::Entrance(p) => p.set(name, value),
            Self::House(p) => p.set(name, value),
        }
        .map_err(|e| format!("{}: {e}", self.body().name()))
    }

    /// Computes the per-frame values of the shader once.
    pub fn prepare(&self, uniforms: &ShaderUniforms, debug: DebugView) -> PreparedShader {
        match self {
            Self::Crown(p) => PreparedShader::Crown(crown::prepare(p, uniforms, debug)),
            Self::Entrance(p) => PreparedShader::Entrance(entrance::prepare(p, uniforms, debug)),
            Self::House(p) => PreparedShader::House(house::prepare(p, uniforms, debug)),
        }
    }
}

/// A body's shader with its per-frame values computed.
#[derive(Debug, Clone)]
pub enum PreparedShader {
    Crown(crown::Frame),
    Entrance(entrance::Frame),
    House(house::Frame),
}

impl PreparedShader {
    pub fn shade(&self, sample: &SurfaceSample) -> Vector3 {
        match self {
            Self::Crown(f) => f.shade(sample),
            Self::Entrance(f) => f.shade(sample),
            Self::House(f) => f.shade(sample),
        }
    }
}

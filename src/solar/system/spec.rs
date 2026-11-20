//! Specifications of the solar system. Values are validated, never
//! corrected: `SystemSpec::validate` reports the first problem.

use crate::celestial::{Body, CelestialMaterial};
use crate::solar::SPHERE_RADIUS;
use crate::solar::planet::PlanetMaterial;
use std::collections::HashSet;

/// Stable identity of a body (also its `--set` prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanetId(pub &'static str);

/// Mesh of a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometrySpec {
    /// The course sphere (`assets/models/sphere.obj`).
    Sphere,
}

/// Geometry attached to a body (rings, crowns, loops); none yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentSpec {}

/// The central star.
#[derive(Debug, Clone, PartialEq)]
pub struct StarSpec {
    pub id: PlanetId,
    pub name: &'static str,
    /// Scale of the sphere mesh; the radius is `scale * SPHERE_RADIUS`.
    pub scale: f32,
    /// Self-rotation, radians per simulated second.
    pub spin: f32,
    pub material: PlanetMaterial,
    /// Gravitational parameter (0: no pull).
    pub mu: f32,
}

/// One planet.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanetSpec {
    pub id: PlanetId,
    pub name: &'static str,
    /// Scale of the sphere mesh; the radius is `scale * SPHERE_RADIUS`
    /// (stored as the scale so the drawn transform is exact).
    pub scale: f32,
    /// Distance from the host center to the planet center.
    pub orbit_radius: f32,
    /// Signed speed along the orbit, world units per simulated second
    /// (positive turns from +X towards +Z).
    pub orbit_speed: f32,
    /// Orbit angle at t = 0, radians.
    pub phase: f32,
    /// Self-rotation, radians per simulated second.
    pub spin: f32,
    /// Seed of the planet's procedural details.
    pub seed: u32,
    /// Gravitational parameter (0: no pull).
    pub mu: f32,
    pub material: PlanetMaterial,
    pub geometry: GeometrySpec,
    pub attachments: Vec<AttachmentSpec>,
    /// Body the planet orbits: `None` is the star.
    pub host: Option<PlanetId>,
}

impl PlanetSpec {
    /// Radius of the planet, world units.
    pub fn radius(&self) -> f32 {
        self.scale * SPHERE_RADIUS
    }

    /// Orbital angular speed, radians per simulated second.
    pub fn angular_speed(&self) -> f32 {
        self.orbit_speed / self.orbit_radius
    }
}

/// Largest orbital speed of the Project 3 planets, world units per second:
/// a third of the spacecraft's cruise (3 u/s), so every planet can be
/// caught without quick travel (`ScaleProfile::max_orbit_speed`).
pub const MAX_ORBIT_LINEAR_SPEED: f32 = 1.0;

/// `orbit_radius` that lets `OrbitLayout::solve` choose the radius (the
/// smallest that keeps the gap, rounded up to 0.5). A scene cannot be
/// built from it directly: `validate` asks for a positive radius.
pub const AUTO_ORBIT: f32 = 0.0;

/// The star and its planets.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemSpec {
    pub star: StarSpec,
    pub planets: Vec<PlanetSpec>,
}

/// Why a specification is invalid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecError {
    DuplicateId(PlanetId),
    NonFinite(&'static str),
    OutOfRange(&'static str, f32),
    UnknownHost(PlanetId),
    /// Valid data the scene cannot build yet.
    Unsupported(&'static str),
    /// A `--set` prefix that names no body of the system.
    UnknownBody,
    /// An orbit whose shell would reach the previous one (or the star's):
    /// `radius` is below the smallest radius `min` that keeps the gap.
    OrbitOverlap {
        planet: PlanetId,
        radius: f32,
        min: f32,
    },
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "duplicate body id '{}'", id.0),
            Self::NonFinite(field) => write!(f, "{field} must be finite"),
            Self::OutOfRange(field, v) => write!(f, "{field} out of range ({v})"),
            Self::UnknownHost(id) => write!(f, "unknown host '{}'", id.0),
            Self::Unsupported(what) => write!(f, "{what} is not supported yet"),
            Self::UnknownBody => write!(f, "no body of the system has this id"),
            Self::OrbitOverlap {
                planet,
                radius,
                min,
            } => write!(
                f,
                "orbit of '{}' ({radius}) reaches its inner neighbor (needs {min})",
                planet.0
            ),
        }
    }
}

impl std::error::Error for SpecError {}

/// `value` finite.
fn finite(field: &'static str, value: f32) -> Result<f32, SpecError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SpecError::NonFinite(field))
    }
}

/// `value` finite and strictly positive.
fn positive(field: &'static str, value: f32) -> Result<(), SpecError> {
    if finite(field, value)? > 0.0 {
        Ok(())
    } else {
        Err(SpecError::OutOfRange(field, value))
    }
}

/// `value` finite and not negative.
fn non_negative(field: &'static str, value: f32) -> Result<(), SpecError> {
    if finite(field, value)? >= 0.0 {
        Ok(())
    } else {
        Err(SpecError::OutOfRange(field, value))
    }
}

/// Body values `--set <id>.<param>=<value>` changes in the spec (the other
/// parameters belong to the body's shader). `planet_seed` is not `seed`,
/// a laboratory 6 shader parameter of its own.
pub const SPEC_PARAMS: [&str; 6] = [
    "radius",
    "orbit_radius",
    "orbit_speed",
    "spin",
    "planet_seed",
    "mu",
];

impl SystemSpec {
    /// Applies `--set <id>.<param>=<value>` when `param` is one of
    /// `SPEC_PARAMS` and returns `Ok(true)`; any other parameter is left to
    /// the shader (`Ok(false)`). Values are checked here (finite; radii
    /// positive; `mu` not negative; the seed a whole number), and the whole
    /// spec is validated again when the scene is built.
    pub fn apply_set(&mut self, name: &str, value: f32) -> Result<bool, SpecError> {
        let Some((id, param)) = name.split_once('.') else {
            return Ok(false);
        };
        let Some(&param) = SPEC_PARAMS.iter().find(|&&p| p == param) else {
            return Ok(false);
        };
        finite(param, value)?;
        match param {
            "radius" | "orbit_radius" => positive(param, value)?,
            "mu" => non_negative(param, value)?,
            "planet_seed" if value < 0.0 || value.fract() != 0.0 || value > u32::MAX as f32 => {
                return Err(SpecError::OutOfRange(param, value));
            }
            _ => {}
        }
        if id == self.star.id.0 {
            let star = &mut self.star;
            match param {
                "radius" => star.scale = value / SPHERE_RADIUS,
                "spin" => star.spin = value,
                "mu" => star.mu = value,
                _ => return Err(SpecError::Unsupported("an orbit or seed of the star")),
            }
            return Ok(true);
        }
        let p = self
            .planets
            .iter_mut()
            .find(|p| p.id.0 == id)
            .ok_or(SpecError::UnknownBody)?;
        match param {
            "radius" => p.scale = value / SPHERE_RADIUS,
            "orbit_radius" => p.orbit_radius = value,
            "orbit_speed" => p.orbit_speed = value,
            "spin" => p.spin = value,
            "planet_seed" => p.seed = value as u32,
            _ => p.mu = value,
        }
        Ok(true)
    }

    /// Checks ids (unique), sizes and orbits (finite, positive), speeds,
    /// phases and spins (finite), `mu` (finite, not negative) and hosts
    /// (an existing planet other than itself).
    pub fn validate(&self) -> Result<(), SpecError> {
        positive("star.scale", self.star.scale)?;
        finite("star.spin", self.star.spin)?;
        non_negative("star.mu", self.star.mu)?;
        let mut ids = HashSet::from([self.star.id]);
        for p in &self.planets {
            if !ids.insert(p.id) {
                return Err(SpecError::DuplicateId(p.id));
            }
            positive("planet.scale", p.scale)?;
            positive("planet.orbit_radius", p.orbit_radius)?;
            finite("planet.orbit_speed", p.orbit_speed)?;
            finite("planet.phase", p.phase)?;
            finite("planet.spin", p.spin)?;
            non_negative("planet.mu", p.mu)?;
        }
        for p in &self.planets {
            if let Some(host) = p.host
                && (host == p.id || !self.planets.iter().any(|q| q.id == host))
            {
                return Err(SpecError::UnknownHost(host));
            }
        }
        Ok(())
    }

    /// The Project 3 system at the scale of `ScaleProfile::project3`: The
    /// King's Crown (radius 5.40) with Crimson Entrance (1.10, orbit 14,
    /// 1 u/s) and House of Cards (1.75, orbit 36, 0.75 u/s) in the order of
    /// the suits. Orbits 20 and 27 stay free for Clover and Hearts (C49,
    /// C61); spins, phases and materials are the pre-project ones.
    pub fn project3() -> Self {
        let mut spec = Self::preproject3();
        spec.star.scale = 5.40 / SPHERE_RADIUS;
        for (p, (radius, orbit, speed)) in spec
            .planets
            .iter_mut()
            .zip([(1.10, 14.0, 1.0), (1.75, 36.0, 0.75)])
        {
            p.scale = radius / SPHERE_RADIUS;
            p.orbit_radius = orbit;
            p.orbit_speed = speed;
        }
        spec
    }

    /// The pre-project system exactly as `SolarScene::default_system` wires
    /// it: the star (scale 1.3, spin 0.05, surface loops off), Crimson
    /// Entrance (scale 0.6, orbit 3.0 at 1.5 u/s, spin 0.6) and House of
    /// Cards (scale 0.9, orbit 6.2 at 1.364 u/s from phase pi, spin 0.35).
    pub fn preproject3() -> Self {
        let mut crown = CelestialMaterial::for_body(Body::KingsCrown);
        crown
            .set("loops", 0.0)
            .expect("loops = 0 is a valid crown parameter");
        let planet = |body: Body, scale, orbit_radius: f32, omega: f32, phase, spin| PlanetSpec {
            id: PlanetId(body.name()),
            name: body.display_name(),
            scale,
            orbit_radius,
            orbit_speed: omega * orbit_radius,
            phase,
            spin,
            seed: 0,
            mu: 0.0,
            material: PlanetMaterial::Lab6(CelestialMaterial::for_body(body)),
            geometry: GeometrySpec::Sphere,
            attachments: Vec::new(),
            host: None,
        };
        Self {
            star: StarSpec {
                id: PlanetId(Body::KingsCrown.name()),
                name: Body::KingsCrown.display_name(),
                scale: 1.3,
                spin: 0.05,
                material: PlanetMaterial::Lab6(crown),
                mu: 0.0,
            },
            planets: vec![
                planet(Body::CrimsonEntrance, 0.6, 3.0, 0.5, 0.0, 0.6),
                planet(
                    Body::HouseOfCards,
                    0.9,
                    6.2,
                    0.22,
                    std::f32::consts::PI,
                    0.35,
                ),
            ],
        }
    }
}

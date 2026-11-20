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
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "duplicate body id '{}'", id.0),
            Self::NonFinite(field) => write!(f, "{field} must be finite"),
            Self::OutOfRange(field, v) => write!(f, "{field} out of range ({v})"),
            Self::UnknownHost(id) => write!(f, "unknown host '{}'", id.0),
            Self::Unsupported(what) => write!(f, "{what} is not supported yet"),
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

impl SystemSpec {
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

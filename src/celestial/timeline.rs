//! Deterministic evidence timelines: each one scripts camera, light and
//! simulated time and enforces the invariant its evidence needs.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Timeline {
    /// Fixed camera and light; only the material evolves (animation).
    #[default]
    Material,
    /// One full camera turn with frozen time (parallax).
    Orbit,
    /// One full turn of the light azimuth with a fixed camera (lighting).
    Light,
    /// A quarter camera turn while the material evolves.
    Showcase,
}

impl Timeline {
    pub const ALL: [Timeline; 4] = [Self::Material, Self::Orbit, Self::Light, Self::Showcase];

    pub fn name(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Orbit => "orbit",
            Self::Light => "light",
            Self::Showcase => "showcase",
        }
    }
}

impl fmt::Display for Timeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Timeline {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|t| t.name() == s)
            .ok_or_else(|| {
                format!("unknown timeline '{s}' (expected material, orbit, light or showcase)")
            })
    }
}

/// Camera, light (radians) and simulated time of one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineState {
    pub azimuth: f32,
    pub elevation: f32,
    pub zoom: f32,
    pub light_azimuth: f32,
    pub light_elevation: f32,
    pub time: f32,
}

/// State at `t` seconds into a recording of `duration` seconds that starts
/// from `base`. `freeze` keeps the simulated time at `base.time`; the orbit
/// timeline always freezes it.
pub fn state_at(
    timeline: Timeline,
    base: TimelineState,
    t: f32,
    duration: f32,
    freeze: bool,
) -> TimelineState {
    let fraction = if duration > 0.0 { t / duration } else { 0.0 };
    let mut s = base;
    let frozen = freeze || timeline == Timeline::Orbit;
    if !frozen {
        s.time = base.time + t;
    }
    match timeline {
        Timeline::Material => {}
        Timeline::Orbit => s.azimuth = base.azimuth + TAU * fraction,
        Timeline::Light => s.light_azimuth = base.light_azimuth + TAU * fraction,
        Timeline::Showcase => s.azimuth = base.azimuth + FRAC_PI_2 * fraction,
    }
    s
}

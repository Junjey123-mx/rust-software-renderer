//! Migrating craters and spade mesas of Crimson Entrance.
//!
//! Each event slot repeats on its own cycle. Its center is seeded per cycle
//! inside a cone around the default view, and its envelope is exactly zero
//! at both ends of the cycle, so relocating never pops. The spade only ever
//! enters the height field (a mesa with fractured edges), never as a decal.

use crate::celestial::entrance::EntranceParams;
use crate::celestial::entrance::terrain::{crater_profile, rim_profile};
use crate::shading::cycle::{cycle, envelope};
use crate::shading::math::smoothstep;
use crate::shading::noise::fbm;
use crate::shading::sdf::spade;
use crate::shading::sphere::{angular_distance, east_north, hash_direction, local_plane};
use raylib::math::Vector3;

pub const CRATER_SLOTS: usize = 3;
pub const SPADE_SLOTS: usize = 2;
/// Crater centers stay within this angle of the default view direction.
pub const CRATER_CONE_DEG: f32 = 65.0;
pub const SPADE_CONE_DEG: f32 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Crater,
    Spade,
}

/// One event in its current cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveEvent {
    pub kind: EventKind,
    pub center: Vector3,
    pub east: Vector3,
    pub north: Vector3,
    /// Phase of the cycle in `[0, 1)`.
    pub tau: f32,
    pub cycle: i64,
    /// Crater depth or mesa height of this frame (0 at cycle boundaries).
    pub amount: f32,
    /// The same envelope normalised to `[0, 1]`.
    pub strength: f32,
    /// Crater angular radius or spade size of this frame.
    pub radius: f32,
}

/// Contribution of one event at a direction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Relief {
    pub height: f32,
    /// Raised rim strength (ash).
    pub rim: f32,
    /// Fracture or crack-ring strength.
    pub fracture: f32,
    /// Molten crater floor glow.
    pub magma: f32,
}

/// The three crater slots (offset by a third of the period) and the two
/// spade slots (offset by half) at time `t`.
pub fn active_events(
    p: &EntranceParams,
    seed: u32,
    focus: Vector3,
    t: f32,
) -> [ActiveEvent; CRATER_SLOTS + SPADE_SLOTS] {
    std::array::from_fn(|slot| {
        if slot < CRATER_SLOTS {
            let period = p.crater_period;
            let (k, tau) = cycle(t, period, slot as f32 * period / 3.0);
            let center = hash_direction(
                seed,
                1 + slot as u32,
                k,
                focus,
                CRATER_CONE_DEG.to_radians(),
            );
            let grow = smoothstep(0.15, 0.45, tau);
            let strength = grow * (1.0 - smoothstep(0.75, 1.0, tau));
            let (east, north) = east_north(center);
            ActiveEvent {
                kind: EventKind::Crater,
                center,
                east,
                north,
                tau,
                cycle: k,
                amount: p.crater_depth * strength,
                strength,
                radius: p.crater_radius * (0.55 + 0.45 * grow),
            }
        } else {
            let j = slot - CRATER_SLOTS;
            let period = p.spade_period;
            let (k, tau) = cycle(t, period, j as f32 * period / 2.0);
            let center = hash_direction(seed, 10 + j as u32, k, focus, SPADE_CONE_DEG.to_radians());
            let (east, north) = east_north(center);
            let strength = envelope(tau, 0.3, 0.25, 0.3);
            ActiveEvent {
                kind: EventKind::Spade,
                center,
                east,
                north,
                tau,
                cycle: k,
                amount: p.spade_height * strength,
                strength,
                radius: p.spade_size,
            }
        }
    })
}

/// Relief of a crater: bowl and rim while it sinks, a crack ring while it
/// opens (`tau < 0.2`) and a molten floor while it is stable.
pub fn crater_relief(n: Vector3, e: &ActiveEvent) -> Relief {
    let d = angular_distance(n, e.center) / e.radius;
    if d > 1.6 {
        return Relief::default();
    }
    let opening = 1.0 - smoothstep(0.1, 0.2, e.tau);
    let ring = (-((d - 0.8) / 0.08).powi(2)).exp() * opening * smoothstep(0.0, 0.03, e.tau);
    let glow = smoothstep(0.42, 0.5, e.tau) * (1.0 - smoothstep(0.7, 0.78, e.tau));
    Relief {
        height: crater_profile(d, e.amount),
        rim: rim_profile(d) * e.strength,
        fracture: ring,
        magma: glow * (1.0 - smoothstep(0.35, 0.7, d)),
    }
}

/// Relief of a spade mesa: plateau of the spade distance field, eroded at
/// the end of its cycle, with fractures along its edges while active.
pub fn spade_relief(n: Vector3, e: &ActiveEvent, seed: u32) -> Relief {
    let Some((u, v)) = local_plane(n, e.center, e.east, e.north) else {
        return Relief::default();
    };
    let size = e.radius.max(1e-3);
    let p = (u / size, v / size);
    if p.0.abs() > 1.2 || p.1.abs() > 1.2 {
        return Relief::default();
    }
    let erosion = smoothstep(0.65, 0.95, e.tau);
    let mut s = spade(p);
    if erosion > 0.0 {
        s += erosion * 0.25 * (0.5 + 0.5 * fbm(n * 9.0, 2, seed.wrapping_add(13)));
    }
    let mesa = 1.0 - smoothstep(-0.03, 0.03, s);
    // Energy signal before emergence, then activity while the mesa stands.
    let energy = smoothstep(0.0, 0.05, e.tau) * (1.0 - smoothstep(0.08, 0.12, e.tau));
    let activity = energy.max(e.strength);
    Relief {
        height: e.amount * mesa,
        rim: 0.0,
        fracture: (-(s / 0.02).powi(2)).exp() * activity,
        magma: 0.0,
    }
}

/// Sum of all event reliefs at a direction (heights add, masks take the max).
pub fn event_relief(n: Vector3, events: &[ActiveEvent], seed: u32) -> Relief {
    let mut total = Relief::default();
    for e in events {
        let r = match e.kind {
            EventKind::Crater => crater_relief(n, e),
            EventKind::Spade => spade_relief(n, e, seed),
        };
        total.height += r.height;
        total.rim = total.rim.max(r.rim);
        total.fracture = total.fracture.max(r.fracture);
        total.magma = total.magma.max(r.magma);
    }
    total
}

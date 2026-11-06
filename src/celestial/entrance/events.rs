//! Spade craters of Crimson Entrance: the planet's single geological event.
//!
//! A spade crater fractures the crust along a ♠, sinks it into a basin of
//! that shape, ignites the exposed floor once the basin is fully open, cools
//! and closes again; then its slot starts over somewhere else. The ♠ only
//! enters the height field and masks derived from the same signed distance,
//! never as a decal. Every stage is exactly 0 at the start of a cycle and
//! after `LIFETIME`, so relocating never pops.
//!
//! Event `m` is born at `m * P / 3` and belongs to slot `m mod 3` (large,
//! medium, small), so the three slots run a third of a period apart.
//!
//! Placement: each new crater tries seeded candidates and keeps the first
//! one whose region of influence stays `event_margin` away from the crater
//! still open and from the three that closed last, so active craters never
//! overlap and a zone is not reused right after it closes. The sequence is
//! replayed from a fixed first event, so every center is a pure function of
//! its index and never moves while visible.

use crate::celestial::entrance::EntranceParams;
use crate::shading::cycle::cycle;
use crate::shading::math::{mixf, smoothstep};
use crate::shading::noise::{fbm, gradient3, hash3};
use crate::shading::sdf::spade;
use crate::shading::sphere::{angular_distance, east_north, hash_direction, local_plane};
use raylib::math::Vector3;

pub const EVENT_SLOTS: usize = 3;
/// Size of the large, medium and small slot relative to `event_size`.
pub const SLOT_SIZE: [f32; EVENT_SLOTS] = [1.0, 0.72, 0.5];
/// Depth of each slot relative to `event_depth`.
pub const SLOT_DEPTH: [f32; EVENT_SLOTS] = [1.0, 0.85, 0.7];
/// Peak incandescence of each slot.
pub const SLOT_GLOW: [f32; EVENT_SLOTS] = [1.0, 0.9, 0.8];
/// Fraction of the period an event is visible; the rest is quiet ground.
pub const LIFETIME: f32 = 0.6;
/// Even events start within this angle of the default view direction, so
/// the opening scene always shows one; odd events roam the whole planet.
pub const FRONT_CONE_DEG: f32 = 55.0;
pub const ROAM_CONE_DEG: f32 = 150.0;
/// Signed spade distance beyond which an event leaves the ground untouched
/// (lip, scorch and fractures all fade out before it).
pub const INFLUENCE: f32 = 0.3;
/// Radius of the ♠ silhouette in spade units (the stem corners).
const SPADE_RADIUS: f32 = 0.61;
/// Amplitude of the noise that erodes the signed distance.
const EROSION: f32 = 0.04;
/// Largest turn of a ♠ in its tangent plane, degrees.
const MAX_TURN_DEG: f32 = 30.0;
/// Earlier events a new crater keeps clear of: the one still open and the
/// three that closed most recently.
pub const HISTORY: usize = 4;
/// Seeded candidates tried before the deterministic fallback.
pub const ATTEMPTS: u32 = 16;
/// Fixed fallback directions (Fibonacci sphere).
const FALLBACK_DIRECTIONS: usize = 64;
/// First event replayed: the oldest one alive at `t = 0`.
pub const ORIGIN: i64 = -2;

/// Envelopes of one life cycle as functions of the phase `tau`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stages {
    /// Fine fractures that outline the future ♠, fading as the floor closes.
    pub fracture: f32,
    /// Sinking (0.05-0.2), full depth, regeneration (0.46-0.6).
    pub depth: f32,
    /// Incandescence: rises only after the full depth (0.22-0.34), peaks
    /// (0.34-0.42) and cools (0.42-0.54).
    pub glow: f32,
    /// Cooled crust spreading over the floor (0.4-0.6).
    pub cooling: f32,
    /// Carbonized rim while the crater is open.
    pub scorch: f32,
}

/// The stages at phase `tau`: all exactly 0 at `tau = 0` and from
/// `LIFETIME` on.
pub fn stages(tau: f32) -> Stages {
    if !(tau > 0.0 && tau < LIFETIME) {
        return Stages::default();
    }
    Stages {
        fracture: smoothstep(0.0, 0.08, tau) * (1.0 - smoothstep(0.5, 0.6, tau)),
        depth: smoothstep(0.05, 0.2, tau) * (1.0 - smoothstep(0.46, 0.6, tau)),
        glow: smoothstep(0.22, 0.34, tau) * (1.0 - smoothstep(0.42, 0.54, tau)),
        cooling: smoothstep(0.4, 0.5, tau) * (1.0 - smoothstep(0.52, 0.6, tau)),
        scorch: smoothstep(0.08, 0.25, tau) * (1.0 - smoothstep(0.5, 0.6, tau)),
    }
}

/// Basin profile in units of the depth for the eroded signed distance `s`:
/// -1 on the floor, steep walls across `0.03 > s > -0.13` and a raised
/// lip of 0.22 just outside the ♠. 0 beyond `INFLUENCE`.
pub fn profile(s: f32) -> f32 {
    let bowl = smoothstep(0.03, -0.13, s);
    let deep = smoothstep(-0.1, -0.24, s);
    -(0.85 * bowl + 0.15 * deep) + 0.22 * lip(s)
}

fn lip(s: f32) -> f32 {
    (-((s - 0.07) / 0.05).powi(2)).exp()
}

/// Angular radius of the region an event of tangent-plane `size` touches.
pub fn reach(size: f32) -> f32 {
    ((SPADE_RADIUS + INFLUENCE + EROSION) * size).atan()
}

/// One spade crater in its current cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpadeCrater {
    /// Global index of the event, born at `index * period / 3`.
    pub index: i64,
    pub slot: usize,
    pub center: Vector3,
    /// Axes of the ♠ in the tangent plane (turned per event).
    pub east: Vector3,
    pub north: Vector3,
    /// Tangent-plane size: one spade unit in plane units.
    pub size: f32,
    /// Basin depth at full sink.
    pub depth: f32,
    /// Peak incandescence.
    pub glow_peak: f32,
    /// Phase of the cycle in `[0, 1)`.
    pub tau: f32,
    pub stages: Stages,
    /// Cosine of `reach(size)`: an early dot-product test.
    pub reach_cos: f32,
    pub seed: u32,
}

/// What an event contributes at one direction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CraterSurface {
    pub height: f32,
    /// Eroded lip of ash and crust around the basin.
    pub rim: f32,
    /// Glowing fractures along and inside the ♠.
    pub fracture: f32,
    /// Molten floor in `[0, 1]`.
    pub lava: f32,
    /// White-hot core of the floor at the peak.
    pub core: f32,
    /// Carbonized rock: scorched band around the ♠ and cooled floor.
    pub char: f32,
}

impl SpadeCrater {
    /// Visible part of the cycle.
    pub fn alive(&self) -> bool {
        self.tau > 0.0 && self.tau < LIFETIME
    }

    /// Current depth of the basin.
    pub fn amount(&self) -> f32 {
        self.depth * self.stages.depth
    }

    /// Spade coordinates of `n` and its eroded signed distance; `None`
    /// while the event is quiet or outside its region of influence.
    fn locate(&self, n: Vector3) -> Option<((f32, f32), f32)> {
        if !self.alive() || n.dot(self.center) < self.reach_cos {
            return None;
        }
        let (u, v) = local_plane(n, self.center, self.east, self.north)?;
        let p = (u / self.size, v / self.size);
        let erosion = gradient3(Vector3::new(7.0 * p.0, 7.0 * p.1, 0.5), self.seed);
        Some((p, spade(p) + EROSION * erosion))
    }

    /// Relief at `n`: only the height, for the parallax and bump lookups.
    pub fn height(&self, n: Vector3) -> f32 {
        let amount = self.amount();
        if amount <= 0.0 {
            return 0.0;
        }
        self.locate(n).map_or(0.0, |(_, s)| profile(s) * amount)
    }

    /// Height and every mask at `n`, evaluated once per fragment.
    pub fn surface(&self, n: Vector3) -> CraterSurface {
        let Some((p, s)) = self.locate(n) else {
            return CraterSurface::default();
        };
        let st = self.stages;
        let q = |k: f32, z: f32| Vector3::new(k * p.0, k * p.1, z);
        // Fractures: the ♠ outline first, then a network over the floor.
        let outline = (-(s / 0.02).powi(2)).exp();
        let network = 1.0 - smoothstep(0.0, 0.07, fbm(q(5.0, 0.3), 2, self.seed ^ 0x1f).abs());
        let fracture = st.fracture
            * outline
                .max(0.8 * network * smoothstep(0.1, -0.05, s))
                .min(1.0);
        // Molten floor away from the walls: flowing veins, white-hot core at
        // the peak, crust spreading in patches while it cools.
        let floor = smoothstep(-0.02, -0.08, s);
        let (mut lava, mut core, mut cooled) = (0.0, 0.0, 0.0);
        if floor > 0.0 && st.glow.max(st.cooling) > 0.0 {
            let drift = 3.0 * self.tau;
            let vein = 0.5 + 0.5 * fbm(q(6.0, drift) + Vector3::new(drift, 0.0, 0.0), 3, self.seed);
            let patches = 0.5 + 0.5 * fbm(q(4.0, 2.1), 2, self.seed ^ 0x2b);
            let keep = smoothstep(st.cooling - 0.12, st.cooling + 0.12, patches);
            lava = floor * st.glow * self.glow_peak * keep * mixf(0.6, 1.0, vein);
            core = lava
                * smoothstep(-0.08, -0.2, s)
                * smoothstep(0.45, 0.8, vein)
                * smoothstep(0.7, 1.0, st.glow);
            cooled = floor * st.cooling * (1.0 - keep);
        }
        let band = smoothstep(-0.01, 0.03, s) * (1.0 - smoothstep(0.08, 0.22, s));
        CraterSurface {
            height: profile(s) * self.amount(),
            rim: lip(s) * st.depth,
            fracture,
            lava,
            core,
            char: (band * st.scorch * mixf(0.75, 1.0, network)).max(cooled),
        }
    }
}

/// Candidate center `attempt` of event `index`: even events in front of the
/// default camera, odd ones anywhere on the planet.
pub fn candidate(seed: u32, index: i64, attempt: u32, focus: Vector3) -> Vector3 {
    let cone = if index.rem_euclid(2) == 0 {
        FRONT_CONE_DEG
    } else {
        ROAM_CONE_DEG
    };
    hash_direction(seed, 50 + attempt, index, focus, cone.to_radians())
}

/// Event `index` at `center` and phase `tau`.
pub fn spade_crater(
    p: &EntranceParams,
    seed: u32,
    index: i64,
    center: Vector3,
    tau: f32,
) -> SpadeCrater {
    let slot = index.rem_euclid(EVENT_SLOTS as i64) as usize;
    let event_seed = hash3(index as i32, (index >> 32) as i32, 41, seed);
    let turn = (event_seed as f32 / u32::MAX as f32 - 0.5) * 2.0 * MAX_TURN_DEG.to_radians();
    let (e, nn) = east_north(center);
    let (st, ct) = turn.sin_cos();
    let size = p.event_size * SLOT_SIZE[slot];
    SpadeCrater {
        index,
        slot,
        center,
        east: e * ct + nn * st,
        north: nn * ct - e * st,
        size,
        depth: p.event_depth * SLOT_DEPTH[slot],
        glow_peak: SLOT_GLOW[slot],
        tau,
        stages: stages(tau),
        reach_cos: reach(size).cos(),
        seed: event_seed,
    }
}

/// Center of one event and the angular radius it occupies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub index: i64,
    pub center: Vector3,
    pub reach: f32,
}

/// Smallest angular gap between a crater at `center` with reach `r` and the
/// `earlier` ones once both reaches and `margin` are subtracted. Not
/// negative = no overlap.
pub fn clearance(center: Vector3, r: f32, earlier: &[Placement], margin: f32) -> f32 {
    earlier
        .iter()
        .map(|e| angular_distance(center, e.center) - (r + e.reach + margin))
        .fold(f32::INFINITY, f32::min)
}

fn fallback_direction(i: usize) -> Vector3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / FALLBACK_DIRECTIONS as f32;
    let r = (1.0 - y * y).sqrt();
    let phi = golden * i as f32;
    Vector3::new(r * phi.cos(), y, r * phi.sin())
}

/// Places every event from `ORIGIN` to `last` in birth order. Each one
/// keeps the first candidate clear of the previous `HISTORY` events; if
/// none of the `ATTEMPTS` is, the fixed direction with the largest
/// clearance is used.
// ponytail: O(last) replay per frame (about 500 events per hour of
// simulated time); checkpoint the sequence if days-long runs matter.
pub fn place_events(p: &EntranceParams, seed: u32, focus: Vector3, last: i64) -> Vec<Placement> {
    let mut placed: Vec<Placement> = Vec::new();
    for index in ORIGIN..=last.max(ORIGIN) {
        let slot = index.rem_euclid(EVENT_SLOTS as i64) as usize;
        let r = reach(p.event_size * SLOT_SIZE[slot]);
        let recent = &placed[placed.len().saturating_sub(HISTORY)..];
        let fits = |c: Vector3| clearance(c, r, recent, p.event_margin);
        let center = (0..ATTEMPTS)
            .map(|a| candidate(seed, index, a, focus))
            .find(|&c| fits(c) >= 0.0)
            .unwrap_or_else(|| {
                (0..FALLBACK_DIRECTIONS)
                    .map(fallback_direction)
                    .max_by(|a, b| fits(*a).total_cmp(&fits(*b)))
                    .expect("fallback directions")
            });
        placed.push(Placement {
            index,
            center,
            reach: r,
        });
    }
    placed
}

/// Global index and phase of the current event of `slot` at time `t`.
pub fn slot_cycle(period: f32, slot: usize, t: f32) -> (i64, f32) {
    let third = period / EVENT_SLOTS as f32;
    let (k, tau) = cycle(t, period, -(slot as f32) * third);
    (EVENT_SLOTS as i64 * k + slot as i64, tau)
}

/// The three spade craters at time `t` (clamped to `t >= 0`), by slot.
pub fn active_events(
    p: &EntranceParams,
    seed: u32,
    focus: Vector3,
    t: f32,
) -> [SpadeCrater; EVENT_SLOTS] {
    let cycles: [(i64, f32); EVENT_SLOTS] =
        std::array::from_fn(|slot| slot_cycle(p.event_period, slot, t.max(0.0)));
    let newest = cycles.iter().map(|c| c.0).max().unwrap_or(0);
    let placed = place_events(p, seed, focus, newest);
    std::array::from_fn(|slot| {
        let (index, tau) = cycles[slot];
        let center = placed[(index - ORIGIN) as usize].center;
        spade_crater(p, seed, index, center, tau)
    })
}

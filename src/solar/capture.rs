//! Orbital capture of the Project 3 ship: a slow ship that comes within a
//! planet's capture shell with gravity on is held on an assisted orbit
//! (local flight that keeps drifting along the nose at a target height);
//! the space bar releases it along the orbit's tangent, carried by the
//! planet's own velocity, and the same planet cannot capture it again
//! until it has left the wider release shell (hysteresis).
//!
//! The states, one at a time: space flight (`Free` in `Space` mode),
//! gravity approach (`Approach`), orbital capture (`Captured`),
//! tangential escape (`Escape`) and local exploration (`Free` in `Local`
//! mode, entered with `L`). It reuses the existing controllers: the orbit
//! is `SolarScene::local_step` with an automatic throttle and height, the
//! escape is `space_step_in` with the throttle held while the space bar
//! is. Off unless `OrbitCapture::enabled` (the live viewer turns it on),
//! so recordings and earlier tests fly as before.

use crate::solar::SolarScene;
use crate::solar::attach::AttachmentKind;
use crate::solar::card_ring::CardRing;
use crate::solar::flight::{
    FlightInput, FlightMode, bank_target, ease_bank, local_frame, space_step_in,
};

/// Seconds a ship has to stay in the capture shell, slow and not
/// receding, before it is captured.
pub const CAPTURE_DWELL: f32 = 0.25;
/// Largest speed relative to the planet that can be captured, times the
/// cruise speed: a ship at light speed flies through.
pub const CAPTURE_SPEED: f32 = 1.25;
/// Throttle the assisted orbit adds along the nose (a lap at the
/// clearance height takes `LOCAL_LAP_SECONDS / ORBIT_DRIFT`).
pub const ORBIT_DRIFT: f32 = 0.6;
/// Longest escape burn, seconds, while the space bar is held.
pub const ESCAPE_SECONDS: f32 = 3.0;
/// Release impulse: at least this times the escape velocity of the height
/// the ship leaves from.
pub const RELEASE_MARGIN: f32 = 1.1;

/// Distances from a planet's center that drive the capture, world units:
/// `floor <= orbit < capture < release`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureZone {
    /// Lowest height of the assisted orbit: above the ground plus the
    /// clearance and outside everything the planet carries (the card ring
    /// of House of Cards), with the ship's radius and a margin.
    pub floor: f32,
    /// Height the assisted orbit settles at.
    pub orbit: f32,
    /// A ship between `floor` and this distance can be captured.
    pub capture: f32,
    /// A ship released by this planet can be captured by it again only
    /// after going beyond this distance.
    pub release: f32,
}

impl CaptureZone {
    /// Around a planet of `radius` whose ground plus clearance reaches
    /// `ground`, whose attachments reach `hazard`, for a ship of
    /// `ship_radius`.
    pub fn around(radius: f32, ground: f32, hazard: f32, ship_radius: f32) -> Self {
        let floor = ground.max(hazard + ship_radius + 0.1 * radius);
        let orbit = floor + 0.25 * radius;
        let capture = orbit + 0.35 * radius;
        Self {
            floor,
            orbit,
            capture,
            release: capture + 0.5 * radius,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OrbitState {
    /// Space flight or local exploration.
    #[default]
    Free,
    /// In a capture shell, slow and not receding, for `dwell` seconds.
    Approach { planet: usize, dwell: f32 },
    /// On the assisted orbit, heading for `orbit` from the center.
    Captured { planet: usize, orbit: f32 },
    /// Released along the tangent, burning for `time` seconds.
    Escape { planet: usize, time: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OrbitCapture {
    pub enabled: bool,
    pub state: OrbitState,
    /// Planet that released the ship and cannot capture it again until it
    /// is beyond its release distance.
    pub blocked: Option<usize>,
}

impl OrbitCapture {
    /// Name of the state for the HUD and the window title, `None` when free.
    pub fn label(&self) -> Option<&'static str> {
        match self.state {
            OrbitState::Free => None,
            OrbitState::Approach { .. } => Some("GRAVITY APPROACH"),
            OrbitState::Captured { .. } => Some("ORBITAL CAPTURE"),
            OrbitState::Escape { .. } => Some("TANGENTIAL ESCAPE"),
        }
    }

    pub fn planet(&self) -> Option<usize> {
        match self.state {
            OrbitState::Free => None,
            OrbitState::Approach { planet, .. }
            | OrbitState::Captured { planet, .. }
            | OrbitState::Escape { planet, .. } => Some(planet),
        }
    }
}

impl SolarScene {
    /// Capture distances around body `index`; `None` for the star, a body
    /// without gravity or the pre-project scale.
    pub fn capture_zone(&self, index: usize) -> Option<CaptureZone> {
        let body = &self.bodies[index];
        if self.scale.is_none() || body.orbit.is_none() || body.mu <= 0.0 {
            return None;
        }
        let radius = self.radius_of(index);
        let relief = self.relief_of(index).map_or(0.0, |r| r.field.max_height());
        let ground = radius * (1.0 + relief) + self.flight_scale(index).clearance;
        // The card ring is the one solid thing a planet carries; the
        // assisted orbit stays outside it (the cards have no collision).
        let ring = self
            .attachments
            .iter()
            .filter(|a| a.kind == AttachmentKind::CardRing)
            .filter(|a| self.body_index(a.host) == Some(index))
            .find_map(|_| self.cards.as_ref().map(CardRing::extent))
            .unwrap_or(0.0);
        Some(CaptureZone::around(
            radius,
            ground,
            radius * ring,
            self.ship.radius(),
        ))
    }

    /// `HUD label - planet name` while approaching, captured or escaping.
    pub fn capture_label(&self) -> Option<String> {
        let label = self.capture.label()?;
        let planet = self.capture.planet()?;
        Some(format!("{label} - {}", self.bodies[planet].name))
    }

    fn distance_to(&self, index: usize) -> f32 {
        (self.ship.position - self.bodies[index].transform.position).length()
    }

    /// `L` while captured: the assisted orbit ends and local exploration
    /// (manual local flight) begins around the same planet. `false` when
    /// not captured.
    pub fn leave_capture_for_local(&mut self) -> bool {
        let OrbitState::Captured { planet, .. } = self.capture.state else {
            return false;
        };
        self.capture.state = OrbitState::Free;
        self.capture.blocked = Some(planet);
        true
    }

    /// The follow-rig controller with capture: runs the frame's flight
    /// (space, local, assisted orbit or escape burn) and moves the state.
    pub fn capture_step(&mut self, input: &FlightInput, dt: f32) {
        // The ship leans into turns and strafes, and back to level.
        self.bank = ease_bank(self.bank, bank_target(input, dt), dt);
        // Leaving the release shell lifts the block.
        if let Some(b) = self.capture.blocked
            && self
                .capture_zone(b)
                .is_none_or(|z| self.distance_to(b) > z.release)
        {
            self.capture.blocked = None;
        }
        match self.capture.state {
            OrbitState::Captured { planet, orbit } => self.captured_step(planet, orbit, input, dt),
            OrbitState::Escape { planet, time } => {
                let zone = self.capture_zone(planet);
                let clear = zone.is_none_or(|z| self.distance_to(planet) > z.release);
                if !input.boost
                    || clear
                    || time >= ESCAPE_SECONDS
                    || self.flight != FlightMode::Space
                {
                    self.capture.state = OrbitState::Free;
                    self.free_step(input, dt);
                    return;
                }
                let burn = FlightInput {
                    throttle: 1.0,
                    ..*input
                };
                self.space_step(&burn, dt);
                self.capture.state = OrbitState::Escape {
                    planet,
                    time: time + dt,
                };
            }
            OrbitState::Free | OrbitState::Approach { .. } => {
                self.free_step(input, dt);
                self.approach_check(dt)
            }
        }
    }

    fn space_step(&mut self, input: &FlightInput, dt: f32) {
        let (sources, shapes) = (self.gravity_sources(), self.collision_shapes());
        space_step_in(&mut self.ship, input, dt, &sources, &shapes);
    }

    fn free_step(&mut self, input: &FlightInput, dt: f32) {
        match self.flight {
            FlightMode::Space => self.space_step(input, dt),
            FlightMode::Local { .. } => self.local_step(input, dt),
        }
    }

    /// After a space-flight move: the nearest planet whose capture shell
    /// holds a slow, not receding ship, for `CAPTURE_DWELL` seconds,
    /// captures it.
    fn approach_check(&mut self, dt: f32) {
        if self.flight != FlightMode::Space || !self.gravity {
            self.capture.state = OrbitState::Free;
            return;
        }
        let ship = self.ship;
        let candidate = (0..self.bodies.len())
            .filter(|&i| Some(i) != self.capture.blocked)
            .filter_map(|i| self.capture_zone(i).map(|z| (i, z)))
            .filter(|(i, z)| {
                let d = self.distance_to(*i);
                let offset = ship.position - self.bodies[*i].transform.position;
                let relative = ship.velocity - self.body_velocity(*i);
                (z.floor..=z.capture).contains(&d)
                    && relative.length() <= CAPTURE_SPEED * ship.move_speed
                    && relative.dot(offset) <= 0.0
            })
            .min_by(|a, b| self.distance_to(a.0).total_cmp(&self.distance_to(b.0)));
        let Some((planet, zone)) = candidate else {
            self.capture.state = OrbitState::Free;
            return;
        };
        let dwell = match self.capture.state {
            OrbitState::Approach { planet: p, dwell } if p == planet => dwell + dt,
            _ => dt,
        };
        if dwell < CAPTURE_DWELL {
            self.capture.state = OrbitState::Approach { planet, dwell };
            return;
        }
        self.flight = FlightMode::Local {
            planet,
            previous_planet_position: self.bodies[planet].transform.position,
        };
        self.capture.state = OrbitState::Captured {
            planet,
            orbit: zone.orbit,
        };
    }

    /// One frame on the assisted orbit, or the release: gravity off or a
    /// mode change ends it, the space bar escapes along the tangent.
    fn captured_step(&mut self, planet: usize, orbit: f32, input: &FlightInput, dt: f32) {
        let held = matches!(self.flight, FlightMode::Local { planet: p, .. } if p == planet);
        let zone = self.capture_zone(planet);
        let (Some(zone), true, true) = (zone, held, self.gravity) else {
            // Quick travel, `L` or `G`: back to free flight, not recaptured
            // by this planet until it is left.
            self.capture.state = OrbitState::Free;
            self.capture.blocked = Some(planet);
            self.free_step(input, dt);
            return;
        };
        let center = self.bodies[planet].transform.position;
        if input.boost {
            let Some(frame) = local_frame(&self.ship, center, self.radius_of(planet)) else {
                return;
            };
            let scale = self.flight_scale(planet);
            // Off along the tangent the ship was drifting on, on top of the
            // planet's own motion, at the orbit's pace or the escape
            // velocity of that height with a margin, whichever is faster:
            // from there the distance only grows. No jump in position, and
            // the nose turns to the tangent.
            let distance = self.distance_to(planet);
            let release = (ORBIT_DRIFT * scale.local_speed)
                .max(RELEASE_MARGIN * (2.0 * self.bodies[planet].mu / distance).sqrt());
            self.flight = FlightMode::Space;
            self.ship.forward = frame.tangent;
            self.ship.up = frame.normal;
            self.ship.renormalize();
            self.ship.velocity = self.body_velocity(planet) + frame.tangent * release;
            self.capture.state = OrbitState::Escape { planet, time: 0.0 };
            self.capture.blocked = Some(planet);
            let burn = FlightInput {
                throttle: 1.0,
                ..*input
            };
            self.space_step(&burn, dt);
            return;
        }
        // E/Q move the target height within the capture band; the ship
        // eases towards it (time constant half a second) while the orbit
        // carries it along the nose.
        let radius = self.radius_of(planet);
        let scale = self.flight_scale(planet);
        let orbit = (orbit + input.radial * scale.radial_speed * dt)
            .clamp(zone.floor, zone.capture - 0.1 * radius);
        let distance = self.distance_to(planet);
        let drive = FlightInput {
            throttle: (input.throttle + ORBIT_DRIFT).clamp(-1.0, 1.0),
            radial: ((orbit - distance) / (0.5 * scale.radial_speed)).clamp(-1.0, 1.0),
            boost: false,
            ..*input
        };
        self.local_step(&drive, dt);
        self.capture.state = OrbitState::Captured { planet, orbit };
    }
}

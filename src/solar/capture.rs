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

use crate::shading::math::smoothstep;
use crate::solar::SolarScene;
use crate::solar::attach::AttachmentKind;
use crate::solar::card_ring::CardRing;
use crate::solar::flight::{
    FlightInput, FlightMode, bank_target, ease_bank, local_frame, space_step_in,
};
use raylib::math::Vector3;

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

    /// `HUD label - planet name` while approaching, captured or escaping,
    /// or in atmospheric flight.
    pub fn capture_label(&self) -> Option<String> {
        if let (Some(_), FlightMode::Local { planet, .. }) = (self.surface_altitude, self.flight) {
            return Some(format!("ATMOSPHERIC FLIGHT - {}", self.bodies[planet].name));
        }
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
    /// (space, local, assisted orbit or escape burn), moves the state and
    /// levels the ship with the planet it is near (`level_attitude`).
    pub fn capture_step(&mut self, input: &FlightInput, dt: f32) {
        // The ship leans into turns and strafes, and back to level.
        self.bank = ease_bank(self.bank, bank_target(input, dt), dt);
        // A change of mode since the last frame (`L`, `G`, quick travel).
        self.note_flight_change();
        self.capture_move(input, dt);
        self.note_flight_change();
        if self.flight == FlightMode::Space {
            self.surface_altitude = None;
        }
        self.level_attitude(dt);
        self.ease_atmosphere_camera(dt);
    }

    /// The move of `capture_step` and the change of state.
    fn capture_move(&mut self, input: &FlightInput, dt: f32) {
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
                // The turbo just left an atmosphere: the escape burn runs.
                if matches!(self.capture.state, OrbitState::Escape { .. }) {
                    return;
                }
                self.approach_check(dt);
                self.atmosphere_entry(input);
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
            FlightMode::Local { planet, .. } if self.capture.enabled => {
                self.atmospheric_step(planet, input, dt)
            }
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

/// Time constant of the attitude levelling near a planet, seconds.
pub const ALIGN_EASE: f32 = 0.6;

impl SolarScene {
    /// Eases the ship's attitude towards the local vertical of planet
    /// `planet`, `weight` (0 to 1) of the full rate: the roll about the
    /// nose that brings `up` closest to the planet's normal and, with
    /// `pitch`, the nose onto the tangent plane. Rotations are the minimal
    /// ones about the nose and the right axis, eased with `ALIGN_EASE`, so
    /// the attitude never jumps or flips; where the nose points straight at
    /// the planet (or away) the roll is undefined and is skipped.
    pub fn align_to_planet(&mut self, planet: usize, weight: f32, pitch: bool, dt: f32) {
        let center = self.bodies[planet].transform.position;
        let offset = self.ship.position - center;
        let length = offset.length();
        if !(length.is_finite() && length > 0.0 && dt > 0.0) || weight.is_nan() || weight <= 0.0 {
            return;
        }
        let n = offset / length;
        let k = weight.min(1.0) * (1.0 - (-dt / ALIGN_EASE).exp());
        if pitch {
            let f = self.ship.forward;
            let t = f - n * f.dot(n);
            if t.length() > 1e-3 {
                let t = t.normalized();
                let axis = f.cross(t);
                if axis.length() > 1e-6 {
                    let angle = axis.length().atan2(f.dot(t));
                    self.ship.rotate(axis.normalized(), k * angle);
                }
            }
        }
        let f = self.ship.forward;
        let target = n - f * n.dot(f);
        if target.length() <= 0.05 {
            return;
        }
        let target = target.normalized();
        let up = self.ship.up;
        let angle = up.cross(target).dot(f).atan2(up.dot(target));
        self.ship.rotate(f, k * angle);
    }

    /// The attitude levelling of the live viewer's controller after this
    /// frame's move: full (roll and pitch) while captured or in atmospheric
    /// flight, roll only while approaching or in plain local flight. In
    /// space flight (the escape burn included) the planet no longer steers
    /// the attitude: only the camera recovery runs, once per departure.
    fn level_attitude(&mut self, dt: f32) {
        match self.capture.state {
            OrbitState::Captured { planet, .. } => self.align_to_planet(planet, 1.0, true, dt),
            OrbitState::Approach { planet, .. } => self.align_to_planet(planet, 0.7, false, dt),
            OrbitState::Escape { .. } => self.recover_space_view(dt),
            OrbitState::Free => match self.flight {
                FlightMode::Local { planet, .. } => {
                    let pitch = self.surface_altitude.is_some();
                    self.align_to_planet(planet, 1.0, pitch, dt);
                }
                FlightMode::Space => self.recover_space_view(dt),
            },
        }
    }
}

/// Height of a planet's atmosphere above its highest relief, radii: space
/// flight below it enters atmospheric flight.
pub const ATMOSPHERE_MARGIN: f32 = 0.3;
/// Lowest height of atmospheric flight above the ground, ship radii (never
/// above the clearance of local flight): low enough that the ground and
/// the horizon fill the view.
pub const SKIM: f32 = 2.5;
/// Exploration height above the ground of atmospheric flight entered from
/// space, in atmospheric floors (`atmosphere_floor`).
pub const EXPLORE_HEIGHT: f32 = 1.5;
/// Time constant of the height controller, seconds.
pub const ALTITUDE_EASE: f32 = 0.45;
/// Time constant of the move to the atmospheric camera framing, seconds.
pub const RIG_EASE: f32 = 0.6;
/// Seconds of the camera recovery after leaving a planet.
pub const RECOVERY_SECONDS: f32 = 0.9;
/// Vertical of space flight: the ship's `up` at the start of the game, the
/// one the follow camera frames in space.
pub const SPACE_UP: Vector3 = Vector3::new(0.0, 1.0, 0.0);

/// The camera recovery after leaving a planet: `progress` from 0 to 1 over
/// `RECOVERY_SECONDS`, from the atmospheric framing `from_blend`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraRecovery {
    pub progress: f32,
    pub from_blend: f32,
}

impl SolarScene {
    /// Radius of planet `planet` (center to ground) right below the ship.
    pub fn ground_below(&self, planet: usize) -> f32 {
        let radius = self.radius_of(planet);
        let t = self.bodies[planet].transform;
        let d = (self.ship.position - t.position).normalized();
        self.relief_of(planet).map_or(radius, |r| {
            radius * r.field.surface_radius(t.to_object_dir(d).normalized())
        })
    }

    /// Lowest height above the ground of atmospheric flight around
    /// `planet`: `SKIM` ship radii, or the local flight clearance if lower.
    pub fn atmosphere_floor(&self, planet: usize) -> f32 {
        (SKIM * self.ship.radius()).min(self.flight_scale(planet).clearance)
    }

    /// Distance from the center of `planet` where its atmosphere begins:
    /// `ATMOSPHERE_MARGIN` radii above its highest relief, so a ship enters
    /// it before reaching the collision sphere of space flight.
    pub fn atmosphere_top(&self, planet: usize) -> f32 {
        let relief = self.relief_of(planet).map_or(0.0, |r| r.field.max_height());
        self.radius_of(planet) * (1.0 + relief + ATMOSPHERE_MARGIN)
    }

    /// Atmospheric flight around `planet`: turns and moves on the tangent
    /// plane (`local_step` with no radial input), then a height controller
    /// eases the height above the ground right below the ship towards
    /// `surface_altitude`, which `E`/`Q` move between the clearance and
    /// the top of the atmosphere. The clearance stays a hard floor, the
    /// ship follows the relief up and down, and nothing pushes it out.
    /// The space bar (turbo) leaves; gravity off leaves too.
    pub fn atmospheric_step(&mut self, planet: usize, input: &FlightInput, dt: f32) {
        if input.boost || !self.gravity {
            self.leave_atmosphere(planet, input, dt);
            return;
        }
        let scale = self.flight_scale(planet);
        let floor = self.atmosphere_floor(planet);
        let ceiling = (self.atmosphere_top(planet) - self.radius_of(planet)).max(floor);
        let current = self.distance_to(planet) - self.ground_below(planet);
        let target = self.surface_altitude.unwrap_or(current);
        // Entered above the atmosphere (from an orbit with `L`): the ship
        // keeps its height, and `E` cannot climb higher than either.
        let target =
            (target + input.radial * scale.radial_speed * dt).clamp(floor, ceiling.max(target));
        self.surface_altitude = Some(target);
        let drive = FlightInput {
            radial: 0.0,
            boost: false,
            ..*input
        };
        self.local_step(&drive, dt);
        let center = self.bodies[planet].transform.position;
        let offset = self.ship.position - center;
        let distance = offset.length();
        if !(distance.is_finite() && distance > 0.0 && dt > 0.0) {
            return;
        }
        let ground = self.ground_below(planet);
        let k = 1.0 - (-dt / ALTITUDE_EASE).exp();
        let eased = (distance + (ground + target - distance) * k).max(ground + floor);
        let up = offset / distance;
        self.ship.position = center + up * eased;
        self.ship.velocity += up * ((eased - distance) / dt);
    }

    /// Enters atmospheric flight around `planet`; `descend` aims for the
    /// exploration height (from space), otherwise the current height.
    pub fn enter_atmosphere(&mut self, planet: usize, descend: bool) {
        let floor = self.atmosphere_floor(planet);
        let current = (self.distance_to(planet) - self.ground_below(planet)).max(floor);
        self.flight = FlightMode::Local {
            planet,
            previous_planet_position: self.bodies[planet].transform.position,
        };
        self.capture.state = OrbitState::Free;
        self.surface_altitude = Some(if descend {
            current.min(EXPLORE_HEIGHT * floor)
        } else {
            current
        });
    }

    /// Space flight with gravity, not at light speed, inside a planet's
    /// atmosphere (`atmosphere_top`) that has not just released the ship:
    /// atmospheric flight around the nearest such planet, at any speed.
    fn atmosphere_entry(&mut self, input: &FlightInput) {
        if self.flight != FlightMode::Space
            || !self.gravity
            || input.boost
            || matches!(self.capture.state, OrbitState::Captured { .. })
        {
            return;
        }
        let nearest = (0..self.bodies.len())
            .filter(|&i| Some(i) != self.capture.blocked && self.capture_zone(i).is_some())
            .filter(|&i| self.distance_to(i) < self.atmosphere_top(i))
            .min_by(|&a, &b| self.distance_to(a).total_cmp(&self.distance_to(b)));
        if let Some(planet) = nearest {
            self.enter_atmosphere(planet, true);
        }
    }

    /// Leaves atmospheric flight: with the turbo, off along the tangent and
    /// upwards at the escape pace of `captured_step` (no jump in position,
    /// the nose on the tangent) and burning while the space bar is held;
    /// with gravity off, drifting with the last step's velocity. Either way
    /// the planet cannot take the ship back until it leaves the release
    /// shell.
    fn leave_atmosphere(&mut self, planet: usize, input: &FlightInput, dt: f32) {
        self.flight = FlightMode::Space;
        self.surface_altitude = None;
        self.capture.blocked = Some(planet);
        if !input.boost {
            self.capture.state = OrbitState::Free;
            self.space_step(input, dt);
            return;
        }
        let center = self.bodies[planet].transform.position;
        let Some(frame) = local_frame(&self.ship, center, self.radius_of(planet)) else {
            return;
        };
        let scale = self.flight_scale(planet);
        let distance = self.distance_to(planet);
        let release = (ORBIT_DRIFT * scale.local_speed)
            .max(RELEASE_MARGIN * (2.0 * self.bodies[planet].mu / distance).sqrt());
        let climb = (frame.tangent + frame.normal * 0.6).normalized();
        self.ship.forward = frame.tangent;
        self.ship.up = frame.normal;
        self.ship.renormalize();
        self.ship.velocity = self.body_velocity(planet) + climb * release;
        self.capture.state = OrbitState::Escape { planet, time: 0.0 };
        let burn = FlightInput {
            throttle: 1.0,
            ..*input
        };
        self.space_step(&burn, dt);
    }

    /// Starts the camera recovery when the ship has just gone from local
    /// (captured or atmospheric) flight to space flight, once per
    /// departure; local flight cancels a recovery under way.
    fn note_flight_change(&mut self) {
        let local = matches!(self.flight, FlightMode::Local { .. });
        if self.was_local && !local {
            self.camera_recovery = Some(CameraRecovery {
                progress: 0.0,
                from_blend: self.atmosphere_blend,
            });
        }
        if local {
            self.camera_recovery = None;
        }
        self.was_local = local;
    }

    /// One frame of the camera recovery: the ship rolls about its nose
    /// towards `SPACE_UP` and the follow rig returns from the atmospheric
    /// framing, on a smoothstep schedule of `RECOVERY_SECONDS` that takes
    /// each frame the matching share of what is left, so the end is exact
    /// at any frame rate. Position, velocity and the nose are untouched (the
    /// escape tangent is kept); with the nose along the vertical the roll
    /// is undefined and waits.
    pub fn recover_space_view(&mut self, dt: f32) {
        let Some(mut r) = self.camera_recovery else {
            return;
        };
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let before = smoothstep(0.0, 1.0, r.progress);
        r.progress += dt / RECOVERY_SECONDS;
        // Float steps that land a hair short of the end finish it.
        if r.progress > 1.0 - 1e-4 {
            r.progress = 1.0;
        }
        let after = smoothstep(0.0, 1.0, r.progress);
        let share = if before >= 1.0 {
            1.0
        } else {
            (after - before) / (1.0 - before)
        };
        let f = self.ship.forward;
        let target = SPACE_UP - f * SPACE_UP.dot(f);
        if target.length() > 0.05 {
            let target = target.normalized();
            let up = self.ship.up;
            let angle = up.cross(target).dot(f).atan2(up.dot(target));
            self.ship.rotate(f, share * angle);
        }
        self.atmosphere_blend = r.from_blend * (1.0 - after);
        self.camera_recovery = (r.progress < 1.0).then_some(r);
    }

    /// Moves the follow camera to the atmospheric framing while in
    /// atmospheric flight (time constant `RIG_EASE`); the recovery brings
    /// it back in space.
    fn ease_atmosphere_camera(&mut self, dt: f32) {
        if self.surface_altitude.is_some() && dt.is_finite() && dt > 0.0 {
            let k = 1.0 - (-dt / RIG_EASE).exp();
            self.atmosphere_blend += (1.0 - self.atmosphere_blend) * k;
        }
    }
}

//! Spacecraft flight controllers: pure functions of the ship state, the
//! input of one frame and the real frame delta (flight keeps working while
//! the simulation is paused).

use crate::shading::sphere::rotate_axis;
use crate::solar::ShipState;
use crate::solar::clock::MAX_STEP;
use crate::solar::input::{Look, SolarAction};
use crate::solar::physics::{GravitySource, advance, gravity_accel};
use crate::solar::ship::TurnState;
use crate::solar::system::ScaleProfile;
use raylib::math::Vector3;

/// Signed axes of one frame, each in `[-1, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlightInput {
    /// `W` = +1, `S` = -1.
    pub throttle: f32,
    /// `D` = +1 (towards the ship's right), `A` = -1.
    pub strafe: f32,
    /// Scripted turn rate: +1 turns the nose left at the ship's turn speed.
    pub yaw: f32,
    /// Scripted pitch rate: +1 lifts the nose.
    pub pitch: f32,
    /// Mouse turn of this frame, radians, added to the scripted turns.
    pub look: Look,
    /// Local flight only: `E` = +1 (away from the planet), `Q` = -1.
    pub radial: f32,
    /// Space flight only: the space bar held ramps the speed up to `BOOST`
    /// times.
    pub boost: bool,
}

impl FlightInput {
    /// Opposite keys cancel out.
    pub fn from_actions(actions: &[SolarAction]) -> Self {
        let axis = |pos: SolarAction, neg: SolarAction| {
            actions.contains(&pos) as i32 as f32 - actions.contains(&neg) as i32 as f32
        };
        Self {
            throttle: axis(SolarAction::Forward, SolarAction::Backward),
            strafe: axis(SolarAction::StrafeRight, SolarAction::StrafeLeft),
            yaw: axis(SolarAction::YawLeft, SolarAction::YawRight),
            pitch: axis(SolarAction::PitchUp, SolarAction::PitchDown),
            radial: axis(SolarAction::Ascend, SolarAction::Descend),
            boost: actions.contains(&SolarAction::Boost),
            look: Look::default(),
        }
    }
}

/// Elevation limits of the nose above the plane normal to the reference up
/// (the orbital plane in space flight, the tangent plane in local flight),
/// radians: the ship never flips over the vertical.
pub const MIN_PITCH: f32 = -80.0 * std::f32::consts::PI / 180.0;
pub const MAX_PITCH: f32 = 80.0 * std::f32::consts::PI / 180.0;

/// Elevation of the nose along the ship's own pitch: 0 when level with the
/// plane normal to `reference_up`, positive with the nose raised.
pub fn nose_elevation(ship: &ShipState, reference_up: Vector3) -> f32 {
    ship.forward
        .dot(reference_up)
        .atan2(ship.up.dot(reference_up))
}

/// Turns `yaw` radians around `reference_up` (so the horizon stays level)
/// and `pitch` radians around the ship's right, keeping the nose elevation
/// within `[MIN_PITCH, MAX_PITCH]`; a nose already outside the range only
/// moves back towards it, never jumps.
pub fn steer(ship: &mut ShipState, reference_up: Vector3, yaw: f32, pitch: f32) {
    if yaw.is_finite() && yaw != 0.0 {
        ship.rotate(reference_up, yaw);
    }
    if pitch.is_finite() && pitch != 0.0 {
        let e = nose_elevation(ship, reference_up);
        let target = (e + pitch).clamp(MIN_PITCH.min(e), MAX_PITCH.max(e));
        ship.pitch(target - e);
    }
}

/// Direction of travel from the throttle along `forward` and the strafe
/// along `right`, scaled down to unit length on a diagonal so `W` + `D` is
/// no faster than `W` alone.
pub fn travel(input: &FlightInput, forward: Vector3, right: Vector3) -> Vector3 {
    let v = forward * input.throttle + right * input.strafe;
    let len = v.length();
    if len > 1.0 { v / len } else { v }
}

/// Up of space flight: the normal of the orbital plane.
pub const SPACE_UP: Vector3 = Vector3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

fn clamp_dt(dt: f32) -> f32 {
    if dt.is_finite() {
        dt.clamp(0.0, MAX_STEP)
    } else {
        0.0
    }
}

/// Speed factor of light speed (space flight with the boost held).
pub const BOOST: f32 = 8.0;
/// Seconds for the boost to build from rest to full light speed, and to
/// fade back once released: the speed never jumps.
pub const BOOST_RISE: f32 = 0.4;
pub const BOOST_FALL: f32 = 0.6;

/// Seconds for the engines to spool between idle and full thrust.
pub const THRUST_EASE: f32 = 0.25;

/// `level` moved towards `target` by at most `dt / rise` going up and
/// `dt / fall` going down.
fn approach(level: f32, target: f32, rise: f32, fall: f32, dt: f32) -> f32 {
    if target > level {
        (level + dt / rise).min(target)
    } else {
        (level - dt / fall).max(target)
    }
}

/// Moves the ship's boost level towards 1 while `held` and towards 0
/// otherwise, at the rise and fall rates. Runs on the real delta, so it
/// never touches the simulation clock and works while paused.
pub fn ramp_boost(ship: &mut ShipState, held: bool, dt: f32) {
    let dt = clamp_dt(dt);
    ship.boost = approach(ship.boost, held as i32 as f32, BOOST_RISE, BOOST_FALL, dt);
}

/// Eases the engine level towards `target` (the length of this frame's
/// travel input, 0 when parked).
pub fn ease_thrust(ship: &mut ShipState, target: f32, dt: f32) {
    let dt = clamp_dt(dt);
    let target = target.clamp(0.0, 1.0);
    ship.thrust = approach(ship.thrust, target, THRUST_EASE, THRUST_EASE, dt);
}

/// Acceleration of full thrust in inertial flight, world units per second
/// squared: the 3 u/s cruise in one second (design).
pub const THRUST_ACCEL: f32 = 3.0;

/// Braking of the inertial dampers without thrust, per second: the
/// velocity falls by `exp(-BRAKE_RATE dt)`, to a fifth in about a second.
pub const BRAKE_RATE: f32 = 1.5;

/// World acceleration of this frame's thrust along the nose and sideways;
/// a diagonal is no stronger than one key (`travel`).
pub fn thrust_accel(input: &FlightInput, ship: &ShipState) -> Vector3 {
    travel(input, ship.forward, ship.right()) * THRUST_ACCEL
}

/// Rate of the mouse turn easing, per second: about 63 % of a mouse
/// movement is applied after 1/12 s (design).
pub const TURN_EASE: f32 = 12.0;

/// Takes the share `1 - exp(-k dt)` of the pending turn out of `turn` and
/// returns it as (yaw, pitch). The share depends on the elapsed time only,
/// so a mouse movement turns the ship alike at any frame rate; it never
/// exceeds the pending turn (`k = 0` turns nothing).
pub fn ease_turn(turn: &mut TurnState, k: f32, dt: f32) -> (f32, f32) {
    let alpha = if k.is_finite() && dt.is_finite() {
        (1.0 - (-k.max(0.0) * dt.max(0.0)).exp()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let applied = (turn.yaw * alpha, turn.pitch * alpha);
    turn.yaw -= applied.0;
    turn.pitch -= applied.1;
    applied
}

/// Turn of this frame around `reference_up`: the scripted rates, plus the
/// mouse look directly (pre-project flight) or eased into (inertial
/// flight). The mouse delta is per frame and never scaled by `dt`.
fn frame_turn(ship: &mut ShipState, input: &FlightInput, dt: f32) -> (f32, f32) {
    let turn = ship.turn_speed * dt;
    let (yaw, pitch) = if ship.inertia {
        ship.turn.yaw += input.look.yaw;
        ship.turn.pitch += input.look.pitch;
        ease_turn(&mut ship.turn, TURN_EASE, dt)
    } else {
        (input.look.yaw, input.look.pitch)
    };
    (input.yaw * turn + yaw, input.pitch * turn + pitch)
}

/// Drops the pending pitch the limits would never let the nose take, so a
/// mouse pushed past the limit does not wind up.
fn clamp_pending_pitch(ship: &mut ShipState, reference_up: Vector3) {
    if ship.inertia {
        let e = nose_elevation(ship, reference_up);
        ship.turn.pitch = ship
            .turn
            .pitch
            .clamp(MIN_PITCH.min(e) - e, MAX_PITCH.max(e) - e);
    }
}

/// Space flight: turn (yaw around the orbital plane's normal, pitch within
/// the limits), then move. Pre-project flight moves along the nose and
/// sideways by `travel * move_speed * dt`, times up to `BOOST` as the boost
/// level ramps. Inertial flight (`ShipState::inertia`) adds the thrust,
/// scaled by the same boost, to the velocity, never above that same speed,
/// and moves by the velocity, on the fixed substeps of `physics::advance`. Without thrust the dampers brake it
/// progressively, or it coasts with the dampers off. No collisions here.
pub fn space_step(ship: &mut ShipState, input: &FlightInput, dt: f32) {
    space_step_in(ship, input, dt, &[]);
}

/// `space_step` with the pull of `sources` added to the thrust on every
/// substep of inertial flight (the pre-project flight ignores it). Local
/// flight never calls it, so gravity is never applied twice.
pub fn space_step_in(
    ship: &mut ShipState,
    input: &FlightInput,
    dt: f32,
    sources: &[GravitySource],
) {
    let dt = clamp_dt(dt);
    let (yaw, pitch) = frame_turn(ship, input, dt);
    steer(ship, SPACE_UP, yaw, pitch);
    clamp_pending_pitch(ship, SPACE_UP);
    ramp_boost(ship, input.boost, dt);
    let boost = 1.0 + (BOOST - 1.0) * ship.boost;
    let way = travel(input, ship.forward, ship.right());
    ease_thrust(ship, way.length(), dt);
    if !ship.inertia {
        ship.position += way * (ship.move_speed * boost * dt);
        return;
    }
    let max = ship.move_speed * boost;
    let thrust = thrust_accel(input, ship) * boost;
    let braking = way.length() == 0.0 && ship.dampers;
    // Semi-implicit Euler on fixed substeps: v += a h, then p += v h.
    advance(ship, dt, |ship, h| {
        let pull = gravity_accel(ship.position, sources);
        let mut velocity = ship.velocity + (thrust + pull) * h;
        if braking {
            velocity *= (-BRAKE_RATE * h).exp();
        }
        let speed = velocity.length();
        if speed > max {
            velocity *= max / speed;
        }
        ship.velocity = velocity;
        ship.position += velocity * h;
    });
}

/// Radial frame of the spacecraft around a planet treated as an ideal
/// sphere of `radius` (no relief, no gravity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame {
    /// Unit direction from the planet center to the ship: the local up.
    pub normal: Vector3,
    /// Distance from the planet center to the ship center.
    pub distance: f32,
    /// `distance - radius`: height of the ship center above the surface.
    pub altitude: f32,
    /// Unit direction of travel on the tangent plane: the nose projected
    /// onto it, or the ship's up when the nose points radially.
    pub tangent: Vector3,
}

/// Tolerance below which a vector is treated as zero.
pub const EPSILON: f32 = 1e-5;

fn unit_or_none(v: Vector3) -> Option<Vector3> {
    let len = v.length();
    (len.is_finite() && len > EPSILON).then(|| v / len)
}

/// Local frame of `ship` around a planet centered at `planet` with
/// `radius`. `None` when the ship sits (almost) at the center or any value
/// is non-finite.
pub fn local_frame(ship: &ShipState, planet: Vector3, radius: f32) -> Option<LocalFrame> {
    let offset = ship.position - planet;
    let distance = offset.length();
    let normal = unit_or_none(offset)?;
    if !radius.is_finite() {
        return None;
    }
    let project = |v: Vector3| v - normal * v.dot(normal);
    let tangent = unit_or_none(project(ship.forward))
        .or_else(|| unit_or_none(project(ship.up)))
        .or_else(|| unit_or_none(project(ship.right())))?;
    Some(LocalFrame {
        normal,
        distance,
        altitude: distance - radius,
        tangent,
    })
}

/// Minimum height of the ship center above a planet's ideal surface in
/// local flight: the ship radius (about 0.32) plus room for the camera.
pub const CLEARANCE: f32 = 0.5;
/// Local flight can start only within this height above the surface.
pub const LOCAL_RANGE: f32 = 3.0;

/// Laps of local flight around a planet of the project scale take about
/// this long at full thrust, simulated seconds.
pub const LOCAL_LAP_SECONDS: f32 = 20.0;

/// Distances and speeds of local flight and quick travel around one body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightScale {
    /// Minimum height of the ship center above the surface.
    pub clearance: f32,
    /// Local flight starts only within this height above the surface.
    pub local_range: f32,
    /// Tangential speed of local flight, world units per second.
    pub local_speed: f32,
    /// Radial speed of local flight (`E`/`Q`), world units per second.
    pub radial_speed: f32,
    /// Height of a quick-travel arrival above the surface.
    pub warp_margin: f32,
}

impl FlightScale {
    /// The pre-project constants, for every body.
    pub const fn legacy() -> Self {
        Self {
            clearance: CLEARANCE,
            local_range: LOCAL_RANGE,
            local_speed: LOCAL_SPEED,
            radial_speed: RADIAL_SPEED,
            warp_margin: crate::solar::travel::WARP_MARGIN,
        }
    }

    /// Around a body of `radius` at the project scale: the clearance is the
    /// ship plus 0.15 R, entry within 2.5 R, arrivals at 1.6 R (between
    /// both), radial moves at 0.35 R/s and a lap at the clearance height in
    /// about `LOCAL_LAP_SECONDS`.
    pub fn for_planet(profile: &ScaleProfile, radius: f32) -> Self {
        let clearance = profile.ship_radius + 0.15 * radius;
        Self {
            clearance,
            local_range: 2.5 * radius,
            local_speed: std::f32::consts::TAU * (radius + clearance) / LOCAL_LAP_SECONDS,
            radial_speed: 0.35 * radius,
            warp_margin: 1.6 * radius,
        }
    }
}

/// Which controller runs this frame; never both.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlightMode {
    #[default]
    Space,
    /// Around the body at `planet` (index into the scene bodies).
    Local {
        planet: usize,
        /// Planet position recorded when it was last followed; set to the
        /// current position on entry so the first displacement is zero.
        previous_planet_position: Vector3,
    },
}

impl FlightMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Local { .. } => "local",
        }
    }
}

/// A body local flight may orbit: its index, current center and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTarget {
    pub index: usize,
    pub position: Vector3,
    pub radius: f32,
}

/// `L`: from local flight, return to space flight keeping the ship as it
/// is; from space flight, enter local flight around the nearest candidate
/// when the ship is outside its clearance and within `LOCAL_RANGE`. On
/// refusal the mode stays `Space` and the reason is returned.
pub fn toggle_local(
    mode: FlightMode,
    ship: &ShipState,
    candidates: &[LocalTarget],
) -> Result<FlightMode, String> {
    toggle_local_scaled(mode, ship, candidates, |_| FlightScale::legacy())
}

/// `toggle_local` with the clearance and entry range of each candidate.
pub fn toggle_local_scaled(
    mode: FlightMode,
    ship: &ShipState,
    candidates: &[LocalTarget],
    scale: impl Fn(&LocalTarget) -> FlightScale,
) -> Result<FlightMode, String> {
    if let FlightMode::Local { .. } = mode {
        return Ok(FlightMode::Space);
    }
    let nearest = candidates
        .iter()
        .min_by(|a, b| {
            let da = (ship.position - a.position).length() - a.radius;
            let db = (ship.position - b.position).length() - b.radius;
            da.total_cmp(&db)
        })
        .ok_or("no planet to fly around")?;
    let distance = (ship.position - nearest.position).length();
    let scale = scale(nearest);
    if !(distance.is_finite() && distance >= nearest.radius + scale.clearance) {
        return Err("too close to the planet: move away before entering local flight".into());
    }
    if distance > nearest.radius + scale.local_range {
        return Err(format!(
            "too far from the nearest planet ({:.2} above its surface, at most {})",
            distance - nearest.radius,
            scale.local_range
        ));
    }
    Ok(FlightMode::Local {
        planet: nearest.index,
        previous_planet_position: nearest.position,
    })
}

/// Tangential speed of local flight, world units per second.
pub const LOCAL_SPEED: f32 = 0.5;
/// Radial speed of local flight (`E`/`Q`), world units per second.
pub const RADIAL_SPEED: f32 = 0.4;

/// Local flight around a planet now at `planet` with `radius`:
///
/// 1. follow the planet's displacement since the last frame (a position
///    difference, never scaled by `dt`);
/// 2. turn around the radial up (left/right) and the ship's right (up/down,
///    within the pitch limits) without changing the altitude;
/// 3. move along a great-circle arc of the current distance (ahead with
///    `W`/`S`, sideways with `A`/`D`, diagonals no faster), rotating the
///    ship basis by the same rotation so its tilt to the local up is kept;
/// 4. change the distance with `E`/`Q`, never below `radius + CLEARANCE`.
///
/// Light speed is a space-flight tool: here the boost fades out and the
/// space bar is ignored, so the surface stays at a precise, constant pace.
/// Does nothing in space flight. No gravity, no collisions.
pub fn local_step(
    ship: &mut ShipState,
    mode: &mut FlightMode,
    planet: Vector3,
    radius: f32,
    input: &FlightInput,
    dt: f32,
) {
    local_step_scaled(
        ship,
        mode,
        planet,
        radius,
        input,
        dt,
        &FlightScale::legacy(),
    );
}

/// `local_step` with the speeds and clearance of `scale`.
pub fn local_step_scaled(
    ship: &mut ShipState,
    mode: &mut FlightMode,
    planet: Vector3,
    radius: f32,
    input: &FlightInput,
    dt: f32,
    scale: &FlightScale,
) {
    let FlightMode::Local {
        previous_planet_position,
        ..
    } = mode
    else {
        return;
    };
    let start = ship.position;
    ship.position += planet - *previous_planet_position;
    *previous_planet_position = planet;
    let dt = clamp_dt(dt);
    local_motion(ship, planet, radius, input, dt, scale);
    // Local flight is kinematic; the ship leaves it with the world velocity
    // of its last step (moving along with the planet), not at rest.
    if dt > 0.0 {
        ship.velocity = (ship.position - start) / dt;
    }
}

/// Turns and moves of `local_step_scaled` once the planet was followed.
fn local_motion(
    ship: &mut ShipState,
    planet: Vector3,
    radius: f32,
    input: &FlightInput,
    dt: f32,
    scale: &FlightScale,
) {
    ramp_boost(ship, false, dt);

    let Some(frame) = local_frame(ship, planet, radius) else {
        return;
    };
    let (yaw, pitch) = frame_turn(ship, input, dt);
    steer(ship, frame.normal, yaw, pitch);
    clamp_pending_pitch(ship, frame.normal);
    let Some(frame) = local_frame(ship, planet, radius) else {
        return;
    };
    let mut normal = frame.normal;
    let side = frame.tangent.cross(frame.normal);
    let way = travel(input, frame.tangent, side);
    ease_thrust(ship, way.length(), dt);
    let arc = way.length() * scale.local_speed * dt;
    if arc != 0.0
        && let Some(axis) = unit_or_none(frame.normal.cross(way))
    {
        let angle = arc / frame.distance;
        normal = rotate_axis(frame.normal, axis, angle).normalized();
        ship.rotate(axis, angle);
    }
    let distance =
        (frame.distance + input.radial * scale.radial_speed * dt).max(radius + scale.clearance);
    ship.position = planet + normal * distance;
}

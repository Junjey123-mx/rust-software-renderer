//! Spacecraft flight controllers: pure functions of the ship state, the
//! input of one frame and the real frame delta (flight keeps working while
//! the simulation is paused).

use crate::shading::sphere::rotate_axis;
use crate::solar::ShipState;
use crate::solar::clock::MAX_STEP;
use crate::solar::input::SolarAction;
use raylib::math::Vector3;

/// Signed axes of one frame, each in `[-1, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlightInput {
    /// `W` = +1, `S` = -1.
    pub throttle: f32,
    /// Left arrow = +1 (nose towards the ship's left), right arrow = -1.
    pub yaw: f32,
    /// Up arrow = +1 (lift the nose), down arrow = -1.
    pub pitch: f32,
    /// Local flight only: `E` = +1 (away from the planet), `Q` = -1.
    pub radial: f32,
}

impl FlightInput {
    /// Opposite keys cancel out.
    pub fn from_actions(actions: &[SolarAction]) -> Self {
        let axis = |pos: SolarAction, neg: SolarAction| {
            actions.contains(&pos) as i32 as f32 - actions.contains(&neg) as i32 as f32
        };
        Self {
            throttle: axis(SolarAction::Forward, SolarAction::Backward),
            yaw: axis(SolarAction::YawLeft, SolarAction::YawRight),
            pitch: axis(SolarAction::PitchUp, SolarAction::PitchDown),
            radial: axis(SolarAction::Ascend, SolarAction::Descend),
        }
    }
}

fn clamp_dt(dt: f32) -> f32 {
    if dt.is_finite() {
        dt.clamp(0.0, MAX_STEP)
    } else {
        0.0
    }
}

/// Space flight: turn around the ship's own axes, then move along the nose
/// by `throttle * move_speed * dt`. No inertia, no collisions.
pub fn space_step(ship: &mut ShipState, input: &FlightInput, dt: f32) {
    let dt = clamp_dt(dt);
    if input.yaw != 0.0 {
        ship.yaw(input.yaw * ship.turn_speed * dt);
    }
    if input.pitch != 0.0 {
        ship.pitch(input.pitch * ship.turn_speed * dt);
    }
    let step = input.throttle * ship.move_speed * dt;
    ship.position += ship.forward * step;
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
    if !(distance.is_finite() && distance >= nearest.radius + CLEARANCE) {
        return Err("too close to the planet: move away before entering local flight".into());
    }
    if distance > nearest.radius + LOCAL_RANGE {
        return Err(format!(
            "too far from the nearest planet ({:.2} above its surface, at most {LOCAL_RANGE})",
            distance - nearest.radius
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
/// 2. turn around the radial up (left/right) and the ship's right (up/down)
///    without changing the altitude;
/// 3. move along a great-circle arc of the current distance, rotating the
///    ship basis by the same rotation so its tilt to the local up is kept;
/// 4. change the distance with `E`/`Q`, never below `radius + CLEARANCE`.
///
/// Does nothing in space flight. No gravity, no collisions.
pub fn local_step(
    ship: &mut ShipState,
    mode: &mut FlightMode,
    planet: Vector3,
    radius: f32,
    input: &FlightInput,
    dt: f32,
) {
    let FlightMode::Local {
        previous_planet_position,
        ..
    } = mode
    else {
        return;
    };
    ship.position += planet - *previous_planet_position;
    *previous_planet_position = planet;
    let dt = clamp_dt(dt);

    let Some(frame) = local_frame(ship, planet, radius) else {
        return;
    };
    if input.yaw != 0.0 {
        ship.rotate(frame.normal, input.yaw * ship.turn_speed * dt);
    }
    if input.pitch != 0.0 {
        ship.pitch(input.pitch * ship.turn_speed * dt);
    }
    let Some(frame) = local_frame(ship, planet, radius) else {
        return;
    };
    let mut normal = frame.normal;
    let arc = input.throttle * LOCAL_SPEED * dt;
    if arc != 0.0
        && let Some(axis) = unit_or_none(frame.normal.cross(frame.tangent))
    {
        let angle = arc / frame.distance;
        normal = rotate_axis(frame.normal, axis, angle).normalized();
        ship.rotate(axis, angle);
    }
    let distance = (frame.distance + input.radial * RADIAL_SPEED * dt).max(radius + CLEARANCE);
    ship.position = planet + normal * distance;
}

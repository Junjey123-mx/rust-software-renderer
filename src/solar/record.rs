//! Headless recording of the solar scene: a scripted flight rendered at a
//! fixed frame rate and streamed as raw RGB24 for ffmpeg. No window opens;
//! stdout carries only frames and every message goes to the log writer.

use crate::app::CliArgs;
use crate::celestial::cli::CelestialArgs;
use crate::pipeline::ShadedOptions;
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::math::hex;
use crate::solar::flight::{FlightInput, FlightMode, space_step};
use crate::solar::input::{Look, SolarAction};
use crate::solar::viewer::{
    HEIGHT, WIDTH, apply_toggles, build_project3_scene, build_scene, fly, solar_mode,
};
use crate::solar::{ShipState, SolarScene};
use raylib::math::Vector3;
use std::error::Error;
use std::io::Write;

/// Scripted timelines of the solar scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SolarTimeline {
    /// Approach with a burst of light speed (the blue afterburner), local
    /// flight over Crimson Entrance, over House of Cards and its ring of
    /// cards, then the whole system from the orbit rig (22 s).
    #[default]
    Flight,
    /// The continuous tour of the delivery video: space flight with a short
    /// burst of light speed, a continuous approach to the first planet,
    /// local flight around it for a full lap and a boosted departure, all in
    /// one scene, one camera rig and without quick travel.
    Tour,
}

impl std::str::FromStr for SolarTimeline {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flight" => Ok(Self::Flight),
            "tour" => Ok(Self::Tour),
            other => Err(format!(
                "unknown timeline '{other}' (expected material, orbit, light, showcase, flight or tour)"
            )),
        }
    }
}

/// Length of the flight script, seconds.
pub const FLIGHT_SECONDS: f32 = 22.0;

/// One-shot events of the flight script and the instant they fire.
const EVENTS: [(f32, SolarAction); 6] = [
    (3.0, SolarAction::Warp(1)),
    (3.0, SolarAction::ToggleLocal),
    (11.0, SolarAction::Warp(2)),
    (11.0, SolarAction::ToggleLocal),
    (17.0, SolarAction::ToggleLocal),
    (17.0, SolarAction::ToggleCamera),
];

/// Actions of the flight script for the frame at `t`, the previous frame
/// being at `previous` (events fire once, in the frame whose interval
/// `(previous, t]` contains them).
pub fn flight_actions(previous: f32, t: f32) -> Vec<SolarAction> {
    let mut actions: Vec<SolarAction> = EVENTS
        .iter()
        .filter(|(at, _)| previous < *at && *at <= t)
        .map(|(_, a)| *a)
        .collect();
    let held: &[SolarAction] = match t {
        t if t < 1.0 => &[SolarAction::Forward],
        // A burst of light speed, then the ship coasts to a stop short of
        // the star while the exhaust fades back to its idle flame.
        t if t < 1.45 => &[SolarAction::Forward, SolarAction::Boost],
        t if t < 3.0 => &[],
        t if t < 5.0 => &[SolarAction::Forward, SolarAction::Descend],
        t if t < 11.0 => &[SolarAction::Forward],
        t if t < 13.0 => &[SolarAction::Forward, SolarAction::Descend],
        t if t < 17.0 => &[SolarAction::Forward, SolarAction::YawLeft],
        _ => &[],
    };
    actions.extend_from_slice(held);
    actions
}

/// Seconds of straight space flight before the tour enters local flight.
pub const TOUR_SPACE_SECONDS: f32 = 6.0;
/// Light-speed burst of the space leg (start and end, seconds).
pub const TOUR_BOOST: (f32, f32) = (1.0, 1.6);
/// Seconds of local flight: more than one full lap around the planet (a lap
/// at the clearance takes about 20 s, `FlightScale::for_planet`), timed so
/// the departure leaves on the side away from the star (a sweep on the
/// Project 3 layout: 25 to 26.5 s keep clear of the crown's shell).
pub const TOUR_LAP_SECONDS: f32 = 26.0;
/// Seconds the departure pitches the nose up before its light-speed burst.
pub const TOUR_CLIMB_SECONDS: f32 = 1.5;
/// Length of the tour script, seconds.
pub const TOUR_SECONDS: f32 = TOUR_SPACE_SECONDS + TOUR_LAP_SECONDS + TOUR_CLIMB_SECONDS + 4.5;
/// Height of the ship center above the planet's surface when the approach
/// ends (inside the local range, outside the clearance).
pub const TOUR_ARRIVAL_ALTITUDE: f32 = 1.2;

/// Actions of the tour for the frame at `t` (previous frame at
/// `previous`). Only flight keys and the local-flight toggle: never quick
/// travel nor a camera switch.
pub fn tour_actions(previous: f32, t: f32) -> Vec<SolarAction> {
    let lap_end = TOUR_SPACE_SECONDS + TOUR_LAP_SECONDS;
    let climb_end = lap_end + TOUR_CLIMB_SECONDS;
    let mut actions: Vec<SolarAction> = [TOUR_SPACE_SECONDS, lap_end]
        .iter()
        .filter(|&&at| previous < at && at <= t)
        .map(|_| SolarAction::ToggleLocal)
        .collect();
    let held: &[SolarAction] = match t {
        t if t < TOUR_BOOST.0 => &[SolarAction::Forward],
        t if t < TOUR_BOOST.1 => &[SolarAction::Forward, SolarAction::Boost],
        t if t < TOUR_SPACE_SECONDS => &[SolarAction::Forward],
        // Down to the clearance in the first seconds of the lap.
        t if t < TOUR_SPACE_SECONDS + 2.0 => &[SolarAction::Forward, SolarAction::Descend],
        t if t < lap_end => &[SolarAction::Forward],
        t if t < climb_end => &[SolarAction::Forward, SolarAction::PitchUp],
        t if t < TOUR_SECONDS => &[SolarAction::Forward, SolarAction::Boost],
        _ => &[],
    };
    actions.extend_from_slice(held);
    actions
}

/// Distance flown by `template` (at rest, same speeds) along its nose over
/// the space leg of the tour at `fps` frames per second (the frames before
/// the local toggle).
fn tour_space_leg(template: &ShipState, fps: u32) -> f32 {
    let dt = 1.0 / fps as f32;
    let mut ship = ShipState::new(Vector3::zero(), template.scale);
    ship.move_speed = template.move_speed;
    let frames = (TOUR_SPACE_SECONDS * fps as f32).round() as usize;
    for f in 0..frames {
        let t = f as f32 / fps as f32;
        let previous = if f == 0 { -1.0 } else { t - dt };
        space_step(
            &mut ship,
            &FlightInput::from_actions(&tour_actions(previous, t)),
            dt,
        );
    }
    ship.position.length()
}

/// Start of the tour: a ship placed in closed form so that flying the
/// space leg in a straight line ends beside the first planet (the first
/// orbiting body), `TOUR_ARRIVAL_ALTITUDE` above its surface, moving
/// tangentially to it: the planet's position at that instant comes from its
/// orbit, so the approach is continuous and needs no quick travel. The
/// line comes in from above the orbital plane, against the planet's
/// motion, and clears the star and the other bodies.
pub fn tour_start(scene: &SolarScene, fps: u32) -> Option<ShipState> {
    let target = scene.local_targets().into_iter().next()?;
    let body = &scene.bodies[target.index];
    let orbit = body.orbit?;
    let dt = 1.0 / fps as f32;
    let frames = (TOUR_SPACE_SECONDS * fps as f32).round();
    let arrival = scene.time() + (frames - 1.0) * dt;
    let planet = orbit.position_at(arrival, scene.center);
    let up = Vector3::new(0.0, 1.0, 0.0);
    let outward = (planet - scene.center).normalized();
    // Against the orbital motion (angles grow from +X towards +Z).
    let along = up.cross(outward) * orbit.speed.signum();
    let dive = 25f32.to_radians();
    let forward = (along * dive.cos() - up * dive.sin()).normalized();
    // Offset from the planet center at arrival, perpendicular to the nose.
    let lift = 30f32.to_radians();
    let side = outward * lift.cos() + up * lift.sin();
    let side = (side - forward * side.dot(forward)).normalized();
    let end = planet + side * (target.radius + TOUR_ARRIVAL_ALTITUDE);
    let distance = tour_space_leg(&scene.ship, fps);
    let mut ship = scene.ship;
    ship.position = end - forward * distance;
    ship.forward = forward;
    ship.up = forward.cross(up).cross(forward).normalized();
    ship.renormalize();
    ship.boost = 0.0;
    ship.thrust = 0.0;
    Some(ship)
}

/// One frame of a scripted timeline on `s`: the frame's actions and
/// toggles, the exact simulated time (held while the script pauses) and
/// the flight controller. Returns the actions and the toggle messages.
pub fn script_frame(
    s: &mut SolarScene,
    timeline: SolarTimeline,
    frame: usize,
    fps: u32,
    start: f32,
    paused_at: &mut Option<f32>,
) -> (Vec<SolarAction>, Vec<String>) {
    let dt = 1.0 / fps as f32;
    let t = frame as f32 / fps as f32;
    let previous = if frame == 0 { -1.0 } else { t - dt };
    let actions = match timeline {
        SolarTimeline::Flight => flight_actions(previous, t),
        SolarTimeline::Tour => tour_actions(previous, t),
    };
    let messages = apply_toggles(s, &actions);
    let sim = if s.clock.paused {
        *paused_at.get_or_insert(start + t)
    } else {
        *paused_at = None;
        start + t
    };
    s.clock.time = sim;
    s.clock.real_delta = dt;
    s.update();
    fly(s, &actions, &[], Look::default(), dt);
    (actions, messages)
}

/// Copy of `scene` ready to play `timeline` at `fps`: the tour moves the
/// ship to its closed-form start in space flight under the follow rig.
pub fn prepare_timeline(scene: &SolarScene, timeline: SolarTimeline, fps: u32) -> SolarScene {
    let mut s = scene.clone();
    if timeline == SolarTimeline::Tour
        && let Some(ship) = tour_start(&s, fps)
    {
        s.ship = ship;
        s.flight = FlightMode::Space;
        s.rig = crate::solar::camera::CameraRig::Follow;
    }
    s
}

/// What to record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarRecordSettings {
    pub timeline: SolarTimeline,
    pub seconds: f32,
    pub fps: u32,
    pub mode: TriangleMode,
    pub options: ShadedOptions,
    pub width: u32,
    pub height: u32,
}

/// Writes `ceil(seconds * fps)` frames of `width * height * 3` bytes to
/// `out` and the script's messages to `log`; returns the frame count. Each
/// frame sits at exactly `start + f / fps` simulated seconds and the ship
/// moves by exactly `1 / fps` real seconds, so two recordings are identical.
pub fn record_solar(
    scene: &SolarScene,
    settings: &SolarRecordSettings,
    out: &mut impl Write,
    log: &mut impl Write,
) -> Result<usize, Box<dyn Error>> {
    if !(settings.seconds > 0.0 && settings.seconds.is_finite()) || settings.fps == 0 {
        return Err("recording needs a positive duration and frame rate".into());
    }
    let frames = (settings.seconds * settings.fps as f32).ceil() as usize;
    let mut s = prepare_timeline(scene, settings.timeline, settings.fps);
    let start = s.time();
    let mut paused_at: Option<f32> = None;
    let mut framebuffer = Framebuffer::new(settings.width, settings.height);
    framebuffer.set_background_color(hex(0x05030a));
    let mut rgb = Vec::with_capacity((settings.width * settings.height * 3) as usize);
    for f in 0..frames {
        let t = f as f32 / settings.fps as f32;
        let (_, messages) = script_frame(
            &mut s,
            settings.timeline,
            f,
            settings.fps,
            start,
            &mut paused_at,
        );
        for message in messages {
            writeln!(log, "{t:6.2} s: {message}")?;
        }
        s.render_frame(&mut framebuffer, settings.mode, settings.options)?;
        rgb.clear();
        for px in framebuffer.to_rgba_bytes().chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
        out.write_all(&rgb)?;
    }
    out.flush()?;
    Ok(frames)
}

/// `--scene solar --record`: streams the timeline to standard output.
pub fn run(cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let timeline = lab6.solar_timeline.unwrap_or_default();
    // The delivery tour flies the Project 3 system; the pre-project flight
    // keeps its own.
    let scene = match timeline {
        SolarTimeline::Tour => build_project3_scene(cli, lab6)?,
        SolarTimeline::Flight => build_scene(cli, lab6)?,
    };
    let settings = SolarRecordSettings {
        timeline,
        seconds: lab6.record.ok_or("--record requires a duration")?,
        fps: lab6.fps.unwrap_or(30),
        mode: solar_mode(cli, lab6),
        options: ShadedOptions {
            threads: lab6.threads.unwrap_or(0),
            ..ShadedOptions::default()
        },
        width: WIDTH,
        height: HEIGHT,
    };
    let mut log = std::io::stderr();
    writeln!(
        log,
        "Recording the solar scene ({:?} timeline, mode {}) for {} s at {} fps as raw rgb24 {}x{} on stdout",
        settings.timeline,
        settings.mode,
        settings.seconds,
        settings.fps,
        settings.width,
        settings.height
    )?;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let frames = record_solar(&scene, &settings, &mut out, &mut log)?;
    writeln!(log, "Recorded {frames} frames")?;
    Ok(())
}

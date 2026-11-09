//! Headless recording of the solar scene: a scripted flight rendered at a
//! fixed frame rate and streamed as raw RGB24 for ffmpeg. No window opens;
//! stdout carries only frames and every message goes to the log writer.

use crate::app::CliArgs;
use crate::celestial::cli::CelestialArgs;
use crate::pipeline::ShadedOptions;
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::math::hex;
use crate::solar::SolarScene;
use crate::solar::input::SolarAction;
use crate::solar::viewer::{HEIGHT, WIDTH, apply_toggles, build_scene, fly, solar_mode};
use std::error::Error;
use std::io::Write;

/// Scripted timelines of the solar scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SolarTimeline {
    /// Approach, local flight over Crimson Entrance, over House of Cards,
    /// then the whole system from the orbit rig (22 s).
    #[default]
    Flight,
}

impl std::str::FromStr for SolarTimeline {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flight" => Ok(Self::Flight),
            other => Err(format!(
                "unknown timeline '{other}' (expected material, orbit, light, showcase or flight)"
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
        t if t < 3.0 => &[SolarAction::Forward],
        t if t < 5.0 => &[SolarAction::Forward, SolarAction::Descend],
        t if t < 11.0 => &[SolarAction::Forward],
        t if t < 13.0 => &[SolarAction::Forward, SolarAction::Descend],
        t if t < 17.0 => &[SolarAction::Forward, SolarAction::YawLeft],
        _ => &[],
    };
    actions.extend_from_slice(held);
    actions
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
    let dt = 1.0 / settings.fps as f32;
    let mut s = scene.clone();
    let start = s.time();
    let mut paused_at: Option<f32> = None;
    let mut framebuffer = Framebuffer::new(settings.width, settings.height);
    framebuffer.set_background_color(hex(0x05030a));
    let mut rgb = Vec::with_capacity((settings.width * settings.height * 3) as usize);
    for f in 0..frames {
        let t = f as f32 / settings.fps as f32;
        let previous = if f == 0 { -1.0 } else { t - dt };
        let actions = match settings.timeline {
            SolarTimeline::Flight => flight_actions(previous, t),
        };
        for message in apply_toggles(&mut s, &actions) {
            writeln!(log, "{t:6.2} s: {message}")?;
        }
        // Exact simulated time per frame (held while the script pauses).
        let sim = if s.clock.paused {
            *paused_at.get_or_insert(start + t)
        } else {
            paused_at = None;
            start + t
        };
        s.clock.time = sim;
        s.clock.real_delta = dt;
        s.update();
        fly(&mut s, &actions, &[], dt);
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
    let scene = build_scene(cli, lab6)?;
    let settings = SolarRecordSettings {
        timeline: lab6.solar_timeline.unwrap_or_default(),
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

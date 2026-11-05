//! Headless recording: renders timeline frames with `render_shaded` and
//! streams raw RGB24 (no alpha) for ffmpeg. No raylib window is created.

use crate::app::CliArgs;
use crate::celestial::cli::CelestialArgs;
use crate::celestial::timeline::{Timeline, TimelineState, state_at};
use crate::celestial::viewer::{build_scene, celestial_mode};
use crate::celestial::{Body, CelestialScene};
use crate::pipeline::ShadedOptions;
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::math::hex;
use std::error::Error;
use std::io::Write;

/// What to record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordSettings {
    pub mode: TriangleMode,
    pub timeline: Timeline,
    pub seconds: f32,
    pub fps: u32,
    pub freeze: bool,
    pub options: ShadedOptions,
    pub width: u32,
    pub height: u32,
}

fn base_state(scene: &CelestialScene) -> TimelineState {
    let camera = &scene.view.camera;
    TimelineState {
        azimuth: camera.azimuth,
        elevation: camera.elevation,
        zoom: camera.zoom,
        light_azimuth: scene.light_azimuth,
        light_elevation: scene.light_elevation,
        time: scene.time,
    }
}

/// Writes `ceil(seconds * fps)` frames of `width * height * 3` bytes to
/// `out` and returns the number of frames. The scene is not modified.
pub fn record(
    scene: &CelestialScene,
    settings: &RecordSettings,
    out: &mut impl Write,
) -> Result<usize, Box<dyn Error>> {
    if !(settings.seconds > 0.0 && settings.seconds.is_finite()) || settings.fps == 0 {
        return Err("recording needs a positive duration and frame rate".into());
    }
    let frames = (settings.seconds * settings.fps as f32).ceil() as usize;
    let base = base_state(scene);
    let mut frame_scene = scene.clone();
    let mut framebuffer = Framebuffer::new(settings.width, settings.height);
    framebuffer.set_background_color(hex(0x120610));
    let mut rgb = Vec::with_capacity((settings.width * settings.height * 3) as usize);
    for f in 0..frames {
        let t = f as f32 / settings.fps as f32;
        let s = state_at(
            settings.timeline,
            base,
            t,
            settings.seconds,
            settings.freeze,
        );
        let camera = &mut frame_scene.view.camera;
        camera.azimuth = s.azimuth;
        camera.elevation = s.elevation;
        camera.zoom = s.zoom;
        frame_scene.light_azimuth = s.light_azimuth;
        frame_scene.light_elevation = s.light_elevation;
        frame_scene.time = s.time;
        framebuffer.clear();
        frame_scene.render(&mut framebuffer, settings.mode, settings.options)?;
        rgb.clear();
        for px in framebuffer.to_rgba_bytes().chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
        out.write_all(&rgb)?;
    }
    out.flush()?;
    Ok(frames)
}

/// `--record`: streams the timeline of `body` to standard output.
pub fn run(body: Body, cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let scene = build_scene(body, cli, lab6)?;
    let settings = RecordSettings {
        mode: celestial_mode(cli, lab6),
        timeline: lab6.timeline.unwrap_or_default(),
        seconds: lab6.record.ok_or("--record requires a duration")?,
        fps: lab6.fps.unwrap_or(30),
        freeze: lab6.freeze_time,
        options: ShadedOptions {
            threads: lab6.threads.unwrap_or(0),
            ..ShadedOptions::default()
        },
        width: crate::celestial::viewer::WIDTH,
        height: crate::celestial::viewer::HEIGHT,
    };
    eprintln!(
        "Recording {} ({} timeline, mode {}) for {} s at {} fps as raw rgb24 {}x{} on stdout",
        body.display_name(),
        settings.timeline,
        settings.mode,
        settings.seconds,
        settings.fps,
        settings.width,
        settings.height
    );
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let frames = record(&scene, &settings, &mut out)?;
    eprintln!("Recorded {frames} frames");
    Ok(())
}

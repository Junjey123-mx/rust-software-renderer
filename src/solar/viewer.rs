//! Window viewer of the solar scene. Raylib only owns the window, reads the
//! keyboard and the relative mouse and presents the CPU framebuffer as a
//! texture.

use crate::app::{CameraAction, CliArgs, apply_input, poll_input, save_screenshot};
use crate::celestial::cli::CelestialArgs;
use crate::celestial::{Body, CelestialScene};
use crate::pipeline::{OrbitCamera, OrbitView, ShadedOptions, Viewport};
use crate::rasterization::TriangleMode;
use crate::rendering::{Framebuffer, Presenter};
use crate::shading::math::hex;
use crate::solar::camera::CameraRig;
use crate::solar::flight::{FlightInput, FlightMode, ramp_boost, space_step};
use crate::solar::input::{
    CAPTURE_KEY, Look, MouseCapture, MouseLook, RELEASE_KEY, SolarAction, poll_solar,
};
use crate::solar::nebula::Nebula;
use crate::solar::skybox::{DEFAULT_SEED, DEFAULT_STARS, StarField};
use crate::solar::travel::warp_to;
use crate::solar::{SolarMeshes, SolarScene};
use raylib::math::Vector3;
use std::error::Error;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// World sphere framed by the orbit camera at zoom 1: the outermost orbit
/// (6.2) plus the space reserved for a ring around House of Cards (1.7).
pub const WORLD_RADIUS: f32 = 8.0;
pub const TITLE: &str = "Rust Software Renderer - Camino al Proyecto 3: Solar";

/// Builds the scene described by the command line: the default system at
/// `--time`, with `--set body.param=value` overrides applied per instance.
pub fn build_scene(cli: &CliArgs, lab6: &CelestialArgs) -> Result<SolarScene, Box<dyn Error>> {
    let camera = OrbitCamera::new(Vector3::zero(), cli.view.unwrap_or_default());
    let view = OrbitView::for_world(camera, WORLD_RADIUS, Viewport::new(WIDTH, HEIGHT, 0.1))?;
    let light = CelestialScene::new(Body::KingsCrown, Default::default(), WIDTH, HEIGHT)?
        .uniforms()
        .light_direction;
    let mut scene = SolarScene::default_system(SolarMeshes::load()?, view, light)?;
    if let Some(t) = lab6.time {
        scene.clock.time = t;
    }
    scene.apply_overrides(&lab6.sets)?;
    scene.camera_mode = lab6.camera.unwrap_or_default();
    scene.quality = lab6.solar_quality.unwrap_or_default();
    let seed = lab6.star_seed.unwrap_or(DEFAULT_SEED);
    scene.nebula = Nebula::new(seed, lab6.nebula.unwrap_or(1.0));
    scene.stars = StarField::generate_with(
        seed,
        lab6.stars.unwrap_or(DEFAULT_STARS),
        lab6.star_density.unwrap_or_default(),
    );
    scene.update();
    Ok(scene)
}

/// Rasterization mode: scanline unless `--mode` was given.
pub fn solar_mode(cli: &CliArgs, lab6: &CelestialArgs) -> TriangleMode {
    if lab6.mode_explicit {
        cli.mode
    } else {
        TriangleMode::FilledScanline
    }
}

/// Window title text: flight mode, the body local flight follows (or the
/// nearest planet in space flight) with the altitude above its ideal
/// surface, the ship speed, the camera rig and the measured frame rate.
pub fn status_line(scene: &SolarScene, speed: f32, fps: f32) -> String {
    let focus = match scene.flight {
        FlightMode::Local { planet, .. } => Some(planet),
        FlightMode::Space => scene
            .local_targets()
            .iter()
            .min_by(|a, b| {
                let da = (scene.ship.position - a.position).length() - a.radius;
                let db = (scene.ship.position - b.position).length() - b.radius;
                da.total_cmp(&db)
            })
            .map(|t| t.index),
    };
    let place = match focus {
        Some(i) => {
            let altitude = (scene.ship.position - scene.bodies[i].transform.position).length()
                - scene.radius_of(i);
            format!(
                "{} altitude {altitude:.2}",
                scene.bodies[i].body().display_name()
            )
        }
        None => "no planet".to_string(),
    };
    format!(
        "Solar | {} flight | {place} | speed {speed:.2} u/s | {} rig | {fps:.0} FPS",
        scene.flight.name(),
        scene.rig.name()
    )
}

/// One frame: clear once, then every body and the spacecraft.
pub fn render_frame(
    scene: &SolarScene,
    framebuffer: &mut Framebuffer,
    mode: TriangleMode,
    options: ShadedOptions,
) -> Result<usize, Box<dyn Error>> {
    Ok(scene.render_frame(framebuffer, mode, options)?)
}

/// Applies the one-shot actions of a frame (pause, camera rig, local
/// flight, warp) and returns what happened, for the caller to report.
pub fn apply_toggles(scene: &mut SolarScene, actions: &[SolarAction]) -> Vec<String> {
    let mut messages = Vec::new();
    for action in actions {
        match action {
            SolarAction::TogglePause => {
                let paused = scene.clock.toggle_pause();
                messages.push(format!(
                    "Time {} at {:.2} s",
                    if paused { "paused" } else { "running" },
                    scene.time()
                ));
            }
            SolarAction::ToggleCamera => {
                scene.rig = scene.rig.toggled();
                messages.push(format!("Camera rig: {}", scene.rig.name()));
            }
            SolarAction::ToggleLocal => messages.push(match scene.toggle_local() {
                Ok(mode) => format!("Flight mode: {}", mode.name()),
                Err(reason) => format!("Local flight unavailable: {reason}"),
            }),
            &SolarAction::Warp(index) if warp_to(scene, index) => {
                messages.push(format!(
                    "Warped to {}",
                    scene.bodies[index].body().display_name()
                ));
            }
            _ => {}
        }
    }
    messages
}

/// Runs the active controller for `dt` real seconds: the spacecraft under
/// the follow rig (space or local flight), the orbit camera otherwise. The
/// mouse `look` of the frame turns whichever is active, unscaled by `dt`.
pub fn fly(
    scene: &mut SolarScene,
    actions: &[SolarAction],
    camera_actions: &[CameraAction],
    look: Look,
    dt: f32,
) {
    match scene.rig {
        CameraRig::Follow => {
            let input = FlightInput {
                look,
                ..FlightInput::from_actions(actions)
            };
            match scene.flight {
                FlightMode::Space => space_step(&mut scene.ship, &input, dt),
                FlightMode::Local { .. } => scene.local_step(&input, dt),
            }
        }
        CameraRig::Orbit => {
            ramp_boost(&mut scene.ship, false, dt);
            // Same senses as the orbit and tilt keys of the earlier scenes.
            scene.view.camera.orbit(-look.yaw);
            scene.view.camera.tilt(look.pitch);
            for &action in camera_actions {
                apply_input(&mut scene.view.camera, action, dt);
            }
        }
    }
}

/// Applies one frame of input in the order of the frame contract: toggles,
/// then the simulation clock and orbits, then the active controller with
/// the real delta (so it keeps working while paused). Returns the messages
/// of the toggles.
pub fn step(
    scene: &mut SolarScene,
    actions: &[SolarAction],
    camera_actions: &[CameraAction],
    look: Look,
    real_dt: f32,
) -> Vec<String> {
    let messages = apply_toggles(scene, actions);
    scene.advance(real_dt);
    let dt = scene.clock.real_delta;
    fly(scene, actions, camera_actions, look, dt);
    messages
}

pub fn run(cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let mut scene = build_scene(cli, lab6)?;
    let mut mode = solar_mode(cli, lab6);
    let options = ShadedOptions {
        threads: lab6.threads.unwrap_or(0),
        ..ShadedOptions::default()
    };
    println!(
        "Solar scene: {} bodies and the spacecraft, mode {mode}, camera {} ({} rig). Controls: mouse steers (follow) or orbits (orbit rig), TAB capture/free mouse, W/S thrust, A/D strafe, SPACE light speed (space flight), L local flight, E/Q altitude (local), F1/F2/F3 warp, V camera rig, Q/E zoom, 1/2/3 mode, R reset, T/F/S views, P pause, ESC free mouse (again: quit)",
        scene.bodies.len(),
        scene.camera_mode.name(),
        scene.rig.name()
    );
    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title(TITLE)
        .build();
    rl.set_target_fps(60);
    // ESC frees the mouse first; only a second ESC quits.
    rl.set_exit_key(None);
    let mouse = mouse_look(lab6);
    let mut capture = MouseCapture::default();
    capture.toggle();
    rl.disable_cursor();
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.set_background_color(hex(0x05030a));
    let mut presenter = Presenter::new(&mut rl, &thread, &framebuffer)?;
    let mut reported = false;
    let mut title_timer = 0.0f32;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        if rl.is_key_pressed(CAPTURE_KEY) {
            let captured = capture.toggle();
            set_cursor(&mut rl, captured);
        }
        if rl.is_key_pressed(RELEASE_KEY) {
            if !capture.release() {
                break;
            }
            set_cursor(&mut rl, false);
        }
        let look = capture.turn(rl.get_mouse_delta(), rl.is_window_focused(), &mouse);
        let actions = poll_solar(&rl);
        let (mut camera_actions, new_mode) = poll_input(&rl);
        // The mouse orbits this scene's camera; the arrows would compete.
        camera_actions.retain(|a| {
            !matches!(
                a,
                CameraAction::OrbitLeft
                    | CameraAction::OrbitRight
                    | CameraAction::TiltUp
                    | CameraAction::TiltDown
            )
        });
        if let Some(m) = new_mode
            && m != mode
        {
            mode = m;
            println!("Mode: {mode}");
        }
        let before = scene.ship.position;
        let mode_before = scene.flight;
        for message in step(&mut scene, &actions, &camera_actions, look, dt) {
            println!("{message}");
        }
        title_timer += dt;
        if title_timer >= 0.5 || scene.flight != mode_before {
            let speed = if scene.clock.real_delta > 0.0 {
                (scene.ship.position - before).length() / scene.clock.real_delta
            } else {
                0.0
            };
            let title = status_line(&scene, speed, rl.get_fps() as f32);
            rl.set_window_title(&thread, &title);
            title_timer = 0.0;
        }
        let shaded = render_frame(&scene, &mut framebuffer, mode, options)?;
        if !reported {
            println!(
                "Rendered {} bodies and the spacecraft, {shaded} fragments shaded",
                scene.bodies.len()
            );
            reported = true;
        }
        presenter.present(&mut rl, &thread, &framebuffer)?;
        if let Some(file) = &cli.screenshot {
            save_screenshot(&mut rl, &thread, file)?;
            println!("Saved window screenshot to {file}");
            break;
        }
    }
    Ok(())
}

/// Mouse steering of the command line: the default sensitivities unless
/// `--mouse-sensitivity` was given, inverted with `--invert-mouse`.
pub fn mouse_look(lab6: &CelestialArgs) -> MouseLook {
    let defaults = MouseLook::default();
    let (sensitivity_x, sensitivity_y) = lab6
        .mouse_sensitivity
        .unwrap_or((defaults.sensitivity_x, defaults.sensitivity_y));
    MouseLook {
        sensitivity_x,
        sensitivity_y,
        invert_y: lab6.invert_mouse,
    }
}

/// Hides and locks the cursor while the mouse steers; shows it otherwise.
fn set_cursor(rl: &mut raylib::RaylibHandle, captured: bool) {
    if captured {
        rl.disable_cursor();
        println!("Mouse captured: it steers (TAB or ESC frees it)");
    } else {
        rl.enable_cursor();
        println!("Mouse free (TAB captures it, ESC quits)");
    }
}

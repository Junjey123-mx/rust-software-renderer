//! Window viewer of the solar scene. Raylib only owns the window, reads the
//! keyboard and presents the CPU framebuffer as a texture.

use crate::app::{CameraAction, CliArgs, apply_input, poll_input, save_screenshot};
use crate::celestial::cli::CelestialArgs;
use crate::celestial::{Body, CelestialScene};
use crate::pipeline::{OrbitCamera, OrbitView, ShadedOptions, Viewport};
use crate::rasterization::TriangleMode;
use crate::rendering::{Framebuffer, Presenter};
use crate::shading::math::hex;
use crate::solar::camera::CameraRig;
use crate::solar::flight::{FlightInput, space_step};
use crate::solar::input::{SolarAction, poll_solar};
use crate::solar::{SolarMeshes, SolarScene};
use raylib::math::Vector3;
use std::error::Error;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// World sphere framed by the orbit camera at zoom 1.
pub const WORLD_RADIUS: f32 = 5.3;
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

/// One frame: clear once, then every body and the spacecraft.
pub fn render_frame(
    scene: &SolarScene,
    framebuffer: &mut Framebuffer,
    mode: TriangleMode,
    options: ShadedOptions,
) -> Result<usize, Box<dyn Error>> {
    Ok(scene.render_frame(framebuffer, mode, options)?)
}

/// Applies one frame of input in the order of the frame contract: toggles,
/// then the simulation clock and orbits, then the active controller (the
/// spacecraft under the follow rig, the orbit camera otherwise), which uses
/// the real delta so it keeps working while paused.
pub fn step(
    scene: &mut SolarScene,
    actions: &[SolarAction],
    camera_actions: &[CameraAction],
    real_dt: f32,
) {
    for action in actions {
        match action {
            SolarAction::TogglePause => {
                let paused = scene.clock.toggle_pause();
                println!(
                    "Time {} at {:.2} s",
                    if paused { "paused" } else { "running" },
                    scene.time()
                );
            }
            SolarAction::ToggleCamera => {
                scene.rig = scene.rig.toggled();
                println!("Camera rig: {}", scene.rig.name());
            }
            _ => {}
        }
    }
    scene.advance(real_dt);
    let dt = scene.clock.real_delta;
    match scene.rig {
        CameraRig::Follow => space_step(&mut scene.ship, &FlightInput::from_actions(actions), dt),
        CameraRig::Orbit => {
            for &action in camera_actions {
                apply_input(&mut scene.view.camera, action, dt);
            }
        }
    }
}

pub fn run(cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let mut scene = build_scene(cli, lab6)?;
    let mut mode = solar_mode(cli, lab6);
    let options = ShadedOptions {
        threads: lab6.threads.unwrap_or(0),
        ..ShadedOptions::default()
    };
    println!(
        "Solar scene: {} bodies and the spacecraft, mode {mode}, camera {} ({} rig). Controls: W/S thrust, arrows turn (follow) or orbit (orbit rig), V camera rig, Q/E zoom, 1/2/3 mode, R reset, T/F/S views, SPACE pause, ESC quit",
        scene.bodies.len(),
        scene.camera_mode.name(),
        scene.rig.name()
    );
    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title(TITLE)
        .build();
    rl.set_target_fps(60);
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.set_background_color(hex(0x05030a));
    let mut presenter = Presenter::new(&mut rl, &thread, &framebuffer)?;
    let mut reported = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let actions = poll_solar(&rl);
        let (camera_actions, new_mode) = poll_input(&rl);
        if let Some(m) = new_mode
            && m != mode
        {
            mode = m;
            println!("Mode: {mode}");
        }
        step(&mut scene, &actions, &camera_actions, dt);
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

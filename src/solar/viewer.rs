//! Window viewer of the solar scene. Raylib only owns the window, reads the
//! keyboard and presents the CPU framebuffer as a texture.

use crate::app::{CliArgs, apply_input, poll_input, save_screenshot};
use crate::celestial::cli::CelestialArgs;
use crate::celestial::{Body, CelestialScene};
use crate::pipeline::{OrbitCamera, OrbitView, ShadedOptions, Transform, Viewport};
use crate::rasterization::TriangleMode;
use crate::rendering::{Framebuffer, Presenter};
use crate::shading::math::hex;
use crate::solar::{BodyInstance, Orbit, SolarMeshes, SolarScene};
use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use std::error::Error;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// World sphere framed by the orbit camera at zoom 1.
pub const WORLD_RADIUS: f32 = 5.3;
pub const TITLE: &str = "Rust Software Renderer - Camino al Proyecto 3: Solar";

/// Builds the scene described by the command line: the star at the center
/// and the two planets on circular orbits.
pub fn build_scene(cli: &CliArgs, _lab6: &CelestialArgs) -> Result<SolarScene, Box<dyn Error>> {
    let camera = OrbitCamera::new(Vector3::zero(), cli.view.unwrap_or_default());
    let view = OrbitView::for_world(camera, WORLD_RADIUS, Viewport::new(WIDTH, HEIGHT, 0.1))?;
    let light = CelestialScene::new(Body::KingsCrown, Default::default(), WIDTH, HEIGHT)?
        .uniforms()
        .light_direction;
    let mut scene = SolarScene::new(SolarMeshes::load()?, view, light);
    scene.bodies = vec![
        BodyInstance::new(Body::KingsCrown, Transform::new(Vector3::zero(), 1.3), 0.05),
        BodyInstance::new(
            Body::CrimsonEntrance,
            Transform::new(Vector3::new(2.6, 0.0, 0.0), 0.6),
            0.6,
        )
        .with_orbit(Orbit::new(2.6, 0.5, 0.0)?),
        BodyInstance::new(
            Body::HouseOfCards,
            Transform::new(Vector3::new(-4.4, 0.0, 0.0), 0.9),
            0.35,
        )
        .with_orbit(Orbit::new(4.4, 0.22, std::f32::consts::PI)?),
    ];
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

/// One frame: clear once, then every body.
pub fn render_frame(
    scene: &SolarScene,
    framebuffer: &mut Framebuffer,
    mode: TriangleMode,
    options: ShadedOptions,
) -> Result<usize, Box<dyn Error>> {
    framebuffer.clear();
    let stats = scene.render_objects_without_clear(framebuffer, mode, options)?;
    Ok(stats.iter().map(|s| s.fragments_shaded).sum())
}

pub fn run(cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let mut scene = build_scene(cli, lab6)?;
    let mut mode = solar_mode(cli, lab6);
    let options = ShadedOptions {
        threads: lab6.threads.unwrap_or(0),
        ..ShadedOptions::default()
    };
    println!(
        "Solar scene: {} bodies, mode {mode}. Controls: arrows orbit/tilt, Q/E zoom, 1/2/3 mode, R reset, T/F/S views, SPACE pause, ESC quit",
        scene.bodies.len()
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
        let (actions, new_mode) = poll_input(&rl);
        for action in actions {
            apply_input(&mut scene.view.camera, action, dt);
        }
        if let Some(m) = new_mode
            && m != mode
        {
            mode = m;
            println!("Mode: {mode}");
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            let paused = scene.clock.toggle_pause();
            println!(
                "Time {} at {:.2} s",
                if paused { "paused" } else { "running" },
                scene.time()
            );
        }
        scene.advance(dt);
        let shaded = render_frame(&scene, &mut framebuffer, mode, options)?;
        if !reported {
            println!(
                "Rendered {} bodies, {shaded} fragments shaded",
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

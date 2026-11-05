//! Window viewer of one celestial body. Raylib only owns the window, reads
//! the keyboard and presents the CPU framebuffer as a texture.

use crate::app::{CliArgs, apply_input, poll_input, save_screenshot};
use crate::celestial::cli::CelestialArgs;
use crate::celestial::{Body, CelestialMaterial, CelestialScene};
use crate::pipeline::ShadedOptions;
use crate::rasterization::TriangleMode;
use crate::rendering::{Framebuffer, Presenter};
use crate::shading::math::hex;
use raylib::consts::KeyboardKey;
use std::error::Error;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// Light steering speed, radians per second.
pub const LIGHT_SPEED: f32 = 1.2;
/// Longest simulated step per frame, so a stall never jumps the animation.
pub const MAX_TIME_STEP: f32 = 0.1;

/// Parameter driven by `[` and `]` for each body.
pub fn main_parameter(body: Body) -> &'static str {
    match body {
        Body::KingsCrown => "corona_pulse_amount",
        Body::CrimsonEntrance => "cloud_coverage",
        Body::HouseOfCards => "vortex_strength",
    }
}

/// Flips a 0/1 switch parameter; `None` when the body does not have it.
pub fn toggle_switch(material: &mut CelestialMaterial, name: &str) -> Option<bool> {
    let on = material.get(name)? < 0.5;
    material.set(name, if on { 1.0 } else { 0.0 }).ok()?;
    Some(on)
}

/// Moves a parameter by `steps` x 5 % of its range, clamped to the range.
pub fn nudge_parameter(material: &mut CelestialMaterial, name: &str, steps: f32) -> Option<f32> {
    let spec = *material.specs().iter().find(|s| s.name == name)?;
    let current = material.get(name)?;
    let next = (current + steps * 0.05 * (spec.max - spec.min)).clamp(spec.min, spec.max);
    material.set(name, next).ok()?;
    Some(next)
}

/// Builds the scene described by the command line.
pub fn build_scene(
    body: Body,
    cli: &CliArgs,
    lab6: &CelestialArgs,
) -> Result<CelestialScene, Box<dyn Error>> {
    let mut scene = CelestialScene::new(body, cli.view.unwrap_or_default(), WIDTH, HEIGHT)?;
    if let Some(t) = lab6.time {
        scene.time = t;
    }
    if let Some(a) = lab6.light_azimuth {
        scene.light_azimuth = a.to_radians();
    }
    if let Some(e) = lab6.light_elevation {
        scene.light_elevation = e.to_radians();
    }
    scene.apply_overrides(&lab6.sets)?;
    if let Some(d) = lab6.debug_view {
        scene.debug = d;
    }
    Ok(scene)
}

/// Rasterization mode of a celestial scene: scanline unless `--mode` given.
pub fn celestial_mode(cli: &CliArgs, lab6: &CelestialArgs) -> TriangleMode {
    if lab6.mode_explicit {
        cli.mode
    } else {
        TriangleMode::FilledScanline
    }
}

fn title(body: Body) -> String {
    format!(
        "Rust Software Renderer - Lab 6 Shaders: {}",
        body.display_name()
    )
}

pub fn run(body: Body, cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), Box<dyn Error>> {
    let mut scene = build_scene(body, cli, lab6)?;
    let mut mode = celestial_mode(cli, lab6);
    let options = ShadedOptions {
        threads: lab6.threads.unwrap_or(0),
        ..ShadedOptions::default()
    };
    println!(
        "{} on the course sphere: mode {mode}, t = {:.2} s, debug view {}",
        body.display_name(),
        scene.time,
        scene.debug.name()
    );
    println!(
        "Controls: arrows orbit/tilt, Q/E zoom, 1/2/3 mode, R reset, T/F/S views, TAB body, SPACE pause, J/L I/K light, N debug view, [ ] main parameter, C/H/B/P effect switches, ESC quit"
    );

    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title(&title(body))
        .build();
    rl.set_target_fps(60);
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.set_background_color(hex(0x120610));
    let mut presenter = Presenter::new(&mut rl, &thread, &framebuffer)?;
    let mut paused = false;
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
        if rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            let next = scene.body().next();
            scene.set_body(next);
            rl.set_window_title(&thread, &title(next));
            println!("Body: {}", next.display_name());
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            paused = !paused;
            println!(
                "Time {} at {:.2} s",
                if paused { "paused" } else { "running" },
                scene.time
            );
        }
        let step = LIGHT_SPEED * dt.clamp(0.0, MAX_TIME_STEP);
        let limit = 89.0_f32.to_radians();
        if rl.is_key_down(KeyboardKey::KEY_J) {
            scene.light_azimuth -= step;
        }
        if rl.is_key_down(KeyboardKey::KEY_L) {
            scene.light_azimuth += step;
        }
        if rl.is_key_down(KeyboardKey::KEY_I) {
            scene.light_elevation = (scene.light_elevation + step).min(limit);
        }
        if rl.is_key_down(KeyboardKey::KEY_K) {
            scene.light_elevation = (scene.light_elevation - step).max(-limit);
        }
        for (key, name) in [
            (KeyboardKey::KEY_C, "clouds"),
            (KeyboardKey::KEY_H, "cloud_shadows"),
            (KeyboardKey::KEY_B, "bump"),
            (KeyboardKey::KEY_P, "parallax"),
        ] {
            if rl.is_key_pressed(key)
                && let Some(on) = toggle_switch(&mut scene.material, name)
            {
                println!("{name}: {}", if on { "on" } else { "off" });
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_N) {
            scene.debug = scene.debug.next();
            println!("Debug view: {}", scene.debug.name());
        }
        for (key, steps) in [
            (KeyboardKey::KEY_LEFT_BRACKET, -1.0),
            (KeyboardKey::KEY_RIGHT_BRACKET, 1.0),
        ] {
            let name = main_parameter(scene.body());
            if rl.is_key_pressed(key)
                && let Some(v) = nudge_parameter(&mut scene.material, name, steps)
            {
                println!("{name} = {v:.3}");
            }
        }

        framebuffer.clear();
        let stats = scene.render(&mut framebuffer, mode, options)?;
        if !reported {
            println!(
                "Rendered {} triangles ({} culled), {} fragments shaded in {:.1} ms raster + {:.1} ms shading",
                stats.triangles_processed,
                stats.triangles_culled,
                stats.fragments_shaded,
                stats.raster_time.as_secs_f64() * 1e3,
                stats.shade_time.as_secs_f64() * 1e3
            );
            reported = true;
        }
        presenter.present(&mut rl, &thread, &framebuffer)?;
        if let Some(file) = &cli.screenshot {
            save_screenshot(&mut rl, &thread, file)?;
            println!("Saved window screenshot to {file}");
            break;
        }
        // Advance after rendering so `--time T --screenshot` shows exactly T.
        if !paused && !lab6.freeze_time {
            scene.time += dt.clamp(0.0, MAX_TIME_STEP);
        }
    }
    Ok(())
}

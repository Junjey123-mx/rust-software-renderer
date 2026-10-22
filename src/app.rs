//! Application lifecycle and orchestration.

use crate::config::AppConfig;
use crate::geometry::Vertex;
use crate::pipeline::{Projection, Uniforms, render};
use crate::rendering::{Framebuffer, Presenter};
use raylib::math::Vector3;
use std::error::Error;

/// Command-line options. `--screenshot <file>` saves the first presented
/// frame of the real window (relative to the working directory) and exits.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub screenshot: Option<String>,
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliArgs, String> {
    let mut out = CliArgs::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--screenshot" => {
                out.screenshot = Some(it.next().ok_or("--screenshot requires a file name")?);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(out)
}

/// Lesson diagnostic triangle in screen coordinates; replaced by the OBJ
/// model once the loader is wired in.
fn diagnostic_triangle() -> Vec<Vertex> {
    [(300.0, 200.0), (400.0, 400.0), (500.0, 200.0)]
        .into_iter()
        .map(|(x, y)| Vertex::new(Vector3::new(x, y, 0.0)))
        .collect()
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args(std::env::args().skip(1))?;
    let config = AppConfig::default();
    let vertex_array = diagnostic_triangle();
    let uniforms = Uniforms::default();

    let (mut rl, thread) = raylib::init()
        .size(config.width as i32, config.height as i32)
        .title(config.window_title)
        .build();
    rl.set_target_fps(config.target_fps);

    let mut framebuffer = Framebuffer::new(config.width, config.height);
    framebuffer.set_background_color(Vector3::new(0.05, 0.05, 0.08));
    let mut presenter = Presenter::new(&mut rl, &thread, &framebuffer)?;

    let mut reported = false;
    while !rl.window_should_close() {
        framebuffer.clear();
        let stats = render(
            &mut framebuffer,
            &uniforms,
            &vertex_array,
            Projection::Screen,
        )?;
        if !reported {
            println!(
                "Rendered {} triangles, {} fragments ({}x{} @ {} FPS)",
                stats.triangles_processed,
                stats.fragments_generated,
                config.width,
                config.height,
                config.target_fps
            );
            reported = true;
        }
        presenter.present(&mut rl, &thread, &framebuffer)?;
        if let Some(file) = &args.screenshot {
            rl.take_screenshot(&thread, file);
            println!("Saved window screenshot to {file}");
            break;
        }
    }
    Ok(())
}

//! Application lifecycle and orchestration.

use crate::config::AppConfig;
use crate::geometry::Vertex;
use crate::io::load_obj;
use crate::pipeline::{Projection, Uniforms, render};
use crate::rendering::{Framebuffer, Presenter};
use raylib::math::Vector3;
use std::error::Error;
use std::path::PathBuf;

/// Command-line options.
///
/// * `--obj <file>`: render another triangulated OBJ instead of the ship.
/// * `--screenshot <file>`: save the first presented frame of the real
///   window (path relative to the working directory) and exit.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub obj: Option<PathBuf>,
    pub screenshot: Option<String>,
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliArgs, String> {
    let mut out = CliArgs::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--obj" => {
                out.obj = Some(PathBuf::from(
                    it.next().ok_or("--obj requires a file path")?,
                ));
            }
            "--screenshot" => {
                out.screenshot = Some(it.next().ok_or("--screenshot requires a file name")?);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(out)
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args(std::env::args().skip(1))?;
    let config = AppConfig::default();
    let model_path = args.obj.unwrap_or(config.model_path.clone());

    // Load once at start-up; the mesh does not change between frames.
    let mesh = load_obj(&model_path)?;
    println!(
        "Loaded {}: {} vertices, {} indices, {} triangles",
        model_path.display(),
        mesh.vertices.len(),
        mesh.indices.len(),
        mesh.triangle_count()
    );
    let vertex_array = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    let uniforms = Uniforms::default();
    let projection = Projection::Fit {
        plane: config.view_plane,
        margin_ratio: config.margin_ratio,
    };

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
        let stats = render(&mut framebuffer, &uniforms, &vertex_array, projection)?;
        if !reported {
            println!(
                "Rendered {} triangles into {} fragments ({:?} view, {}x{} @ {} FPS)",
                stats.triangles_processed,
                stats.fragments_generated,
                config.view_plane,
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

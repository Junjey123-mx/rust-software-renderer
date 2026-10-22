//! Application configuration.

use crate::pipeline::ViewPlane;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
    pub window_title: &'static str,
    /// Default model, resolved relative to the crate so `cargo run` works
    /// from any working directory.
    pub model_path: PathBuf,
    pub view_plane: ViewPlane,
    pub margin_ratio: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            target_fps: 60,
            window_title: "Rust Software Renderer - Lab 4",
            model_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets/models/Laboratorio3.obj"),
            view_plane: ViewPlane::XZ,
            margin_ratio: 0.1,
        }
    }
}

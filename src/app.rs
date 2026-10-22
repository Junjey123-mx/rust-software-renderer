//! Application lifecycle and orchestration.

use crate::config::AppConfig;

pub fn run() {
    let config = AppConfig::default();

    println!("Rust Software Renderer");
    println!("Initial application scaffold.");
    println!("Resolution: {}x{}", config.width, config.height);
    println!("Target FPS: {}", config.target_fps);
}

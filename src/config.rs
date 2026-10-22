//! Application configuration.

#[derive(Debug, Clone, Copy)]
pub struct AppConfig {
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            target_fps: 60,
        }
    }
}

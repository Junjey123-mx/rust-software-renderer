//! Presents the CPU framebuffer in a raylib window.
//!
//! Raylib is used only to own the window and to blit one texture whose
//! pixels come from `Framebuffer::to_rgba_bytes`. No raylib model loading or
//! primitive drawing is involved in producing the image.

use crate::rendering::Framebuffer;
use raylib::prelude::*;

pub struct Presenter {
    texture: Texture2D,
}

impl Presenter {
    /// Creates a texture with the framebuffer's dimensions (RGBA8).
    pub fn new(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        framebuffer: &Framebuffer,
    ) -> Result<Self, String> {
        let image = Image::gen_image_color(
            framebuffer.width() as i32,
            framebuffer.height() as i32,
            Color::BLACK,
        );
        let texture = rl
            .load_texture_from_image(thread, &image)
            .map_err(|e| e.to_string())?;
        Ok(Self { texture })
    }

    /// Uploads the framebuffer pixels and draws the texture at the origin.
    pub fn present(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        framebuffer: &Framebuffer,
    ) -> Result<(), String> {
        self.texture
            .update_texture(&framebuffer.to_rgba_bytes())
            .map_err(|e| e.to_string())?;
        let mut d = rl.begin_drawing(thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&self.texture, 0, 0, Color::WHITE);
        Ok(())
    }
}

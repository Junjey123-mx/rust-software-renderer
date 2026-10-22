//! CPU framebuffer: a row-major grid of RGB colors plus an optional depth
//! buffer. `point` is the laboratory 4/5 path (last write wins, depth
//! untouched); `point_depth` keeps the nearest fragment (smallest depth).

use raylib::math::Vector3;

#[derive(Debug, Clone)]
pub struct Framebuffer {
    width: u32,
    height: u32,
    background: Vector3,
    pixels: Vec<Vector3>,
    depth: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let background = Vector3::zero();
        Self {
            width,
            height,
            background,
            pixels: vec![background; (width * height) as usize],
            depth: vec![f32::INFINITY; (width * height) as usize],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn background_color(&self) -> Vector3 {
        self.background
    }

    pub fn set_background_color(&mut self, color: Vector3) {
        self.background = color;
    }

    /// Fills every pixel with the background color and resets the depth
    /// buffer to "infinitely far".
    pub fn clear(&mut self) {
        self.pixels.fill(self.background);
        self.clear_depth();
    }

    pub fn clear_depth(&mut self) {
        self.depth.fill(f32::INFINITY);
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Writes a pixel; coordinates outside the image are discarded.
    /// The last write to a coordinate wins.
    pub fn point(&mut self, x: i32, y: i32, color: Vector3) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color;
        }
    }

    /// Depth-tested write: the pixel is updated only when `depth` is finite
    /// and strictly smaller than the stored depth (ties keep the first
    /// fragment). Out-of-range coordinates are discarded.
    pub fn point_depth(&mut self, x: i32, y: i32, color: Vector3, depth: f32) {
        if !depth.is_finite() {
            return;
        }
        if let Some(i) = self.index(x, y)
            && depth < self.depth[i]
        {
            self.pixels[i] = color;
            self.depth[i] = depth;
        }
    }

    pub fn get_depth(&self, x: i32, y: i32) -> Option<f32> {
        self.index(x, y).map(|i| self.depth[i])
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Option<Vector3> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Number of pixels whose color differs from the background.
    pub fn count_non_background(&self) -> usize {
        self.pixels
            .iter()
            .filter(|&&p| p != self.background)
            .count()
    }

    /// Converts the image to packed RGBA8 bytes, row-major, alpha opaque.
    /// Each channel is clamped to `0..=1` and rounded to `0..=255` exactly once.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            bytes.extend_from_slice(&[channel(p.x), channel(p.y), channel(p.z), 255]);
        }
        bytes
    }
}

fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

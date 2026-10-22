//! CPU line rasterizer (integer Bresenham, all octants).

use crate::rendering::Fragment;
use raylib::math::{Vector2, Vector3};

/// Generates one fragment per pixel along the segment `start..=end`.
///
/// Both endpoints are included so triangle edges close. Coordinates are
/// rounded to the nearest pixel first, as in the course lesson. Non-finite
/// inputs produce no fragments. Nothing is written to any framebuffer here.
pub fn line(start: Vector2, end: Vector2, color: Vector3) -> Vec<Fragment> {
    if ![start.x, start.y, end.x, end.y]
        .iter()
        .all(|c| c.is_finite())
    {
        return Vec::new();
    }
    let (mut x, mut y) = (start.x.round() as i32, start.y.round() as i32);
    let (x2, y2) = (end.x.round() as i32, end.y.round() as i32);

    let dx = (x2 - x).abs();
    let dy = -(y2 - y).abs();
    let sx = if x < x2 { 1 } else { -1 };
    let sy = if y < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut fragments = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        fragments.push(Fragment::new(x as f32, y as f32, color, 0.0));
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    fragments
}

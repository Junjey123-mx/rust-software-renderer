//! Comets of the Project 3 sky: a few bodies that really travel through the
//! system on wide tilted ellipses around the star, parametrized by the
//! simulated time (deterministic, frozen while paused). Each one is a
//! glowing nucleus and a tail of fixed length that points away from the
//! star, bent a little against the motion. Nothing is stored per frame: the
//! tail is sampled from the current position, so its memory never grows.
//!
//! They are drawn after the scene with the depth buffer (a planet in front
//! hides them), only under the perspective camera, and a comet whose whole
//! tail falls outside the frame is skipped.

use crate::pipeline::PerspectiveCamera;
use crate::rendering::Framebuffer;
use raylib::math::Vector3;

/// Samples along each tail.
pub const TAIL_SAMPLES: usize = 32;

/// One comet on its ellipse: `center + u a cos e + v b sin e`, with
/// `e = phase + speed t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comet {
    pub center: Vector3,
    /// Unit axes of the ellipse's plane.
    pub u: Vector3,
    pub v: Vector3,
    /// Semi-axes along `u` and `v`, world units.
    pub a: f32,
    pub b: f32,
    /// Radians of the parameter per simulated second.
    pub speed: f32,
    pub phase: f32,
    /// Tail length, world units.
    pub tail: f32,
    pub color: Vector3,
}

impl Comet {
    pub fn position(&self, t: f32) -> Vector3 {
        let e = self.phase + self.speed * t;
        self.center + self.u * (self.a * e.cos()) + self.v * (self.b * e.sin())
    }

    /// Velocity, world units per simulated second.
    pub fn velocity(&self, t: f32) -> Vector3 {
        let e = self.phase + self.speed * t;
        (self.u * (-self.a * e.sin()) + self.v * (self.b * e.cos())) * self.speed
    }

    /// Unit direction of the tail: away from the star at `star`, bent a
    /// quarter of the way against the motion.
    pub fn tail_dir(&self, t: f32, star: Vector3) -> Vector3 {
        let away = (self.position(t) - star).normalized();
        let back = -self.velocity(t).normalized();
        let d = away * 0.75 + back * 0.25;
        if d.length() > 1e-4 {
            d.normalized()
        } else {
            away
        }
    }

    /// Point `s` (0 at the nucleus, 1 at the end) of the tail at time `t`;
    /// the far end lags behind along the orbit, so the tail curves.
    pub fn tail_point(&self, t: f32, star: Vector3, s: f32) -> Vector3 {
        let lag = self.velocity(t) * (-0.6 * s * s);
        self.position(t) + self.tail_dir(t, star) * (self.tail * s) + lag
    }
}

/// The comets of a scene.
#[derive(Debug, Clone, PartialEq)]
pub struct CometField {
    pub comets: Vec<Comet>,
}

/// Rotates `v` around the unit `axis` by `angle` (Rodrigues).
fn rotate(v: Vector3, axis: Vector3, angle: f32) -> Vector3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

impl CometField {
    /// Three comets around `star` for the Project 3 system (planets up to
    /// orbit 49): ellipses 45-75 degrees off the orbital plane whose near
    /// end passes 18-30 units from the star, at 2-5 u/s, in a cyan, a
    /// violet and a gold tint.
    pub fn project3(star: Vector3) -> Self {
        let y = Vector3::new(0.0, 1.0, 0.0);
        let data = [
            // (turn of the major axis, tilt, a, b, near, speed, phase, tail, color)
            (
                0.4_f32,
                0.85_f32,
                85.0_f32,
                40.0_f32,
                20.0_f32,
                0.045_f32,
                2.2_f32,
                11.0_f32,
                Vector3::new(0.55, 0.9, 1.0),
            ),
            (
                2.6,
                1.15,
                95.0,
                50.0,
                28.0,
                0.035,
                3.6,
                13.0,
                Vector3::new(0.85, 0.55, 1.0),
            ),
            (
                4.4,
                0.95,
                75.0,
                35.0,
                24.0,
                0.05,
                1.2,
                9.0,
                Vector3::new(1.0, 0.85, 0.5),
            ),
        ];
        let comets = data
            .iter()
            .map(|&(turn, tilt, a, b, near, speed, phase, tail, color)| {
                let u0 = Vector3::new(turn.cos(), 0.0, turn.sin());
                let v = rotate(y.cross(u0).normalized(), u0, tilt);
                let u = u0;
                // The star sits on the major axis, `near` inside the near end.
                let center = star + u * (a - near);
                Comet {
                    center,
                    u,
                    v: v.normalized(),
                    a,
                    b,
                    speed,
                    phase,
                    tail,
                    color,
                }
            })
            .collect();
        Self { comets }
    }
}

/// Adds `color` to the pixel when `depth` is in front of what is drawn
/// there; returns whether it did.
fn add_behind_test(fb: &mut Framebuffer, x: i32, y: i32, depth: f32, color: Vector3) -> bool {
    match (fb.get_depth(x, y), fb.get_pixel(x, y)) {
        (Some(d), Some(p)) if depth < d => {
            let sum = p + color;
            fb.point(
                x,
                y,
                Vector3::new(sum.x.min(1.0), sum.y.min(1.0), sum.z.min(1.0)),
            );
            true
        }
        _ => false,
    }
}

/// A soft disc of `radius` pixels around `(x, y)`, light fading to its rim,
/// added where `depth` is in front of what is drawn. Returns pixels lit.
pub fn glow(
    fb: &mut Framebuffer,
    x: f32,
    y: f32,
    radius: f32,
    depth: f32,
    color: Vector3,
) -> usize {
    let r = radius.ceil() as i32;
    let mut n = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = (dx * dx + dy * dy) as f32 / (radius * radius);
            if d2 <= 1.0 {
                let fade = (1.0 - d2) * (1.0 - d2);
                n += usize::from(add_behind_test(
                    fb,
                    x as i32 + dx,
                    y as i32 + dy,
                    depth,
                    color * fade,
                ));
            }
        }
    }
    n
}

/// Draws the comets at simulated time `t` seen by `camera`, behind what the
/// depth buffer already holds. Returns the comets drawn (the others were
/// off the frame or behind the camera).
pub fn draw_comets(
    fb: &mut Framebuffer,
    field: &CometField,
    camera: &PerspectiveCamera,
    star: Vector3,
    t: f32,
) -> usize {
    let (w, h) = (fb.width(), fb.height());
    let (wf, hf) = (w as f32, h as f32);
    let on_screen = |p: &Vector3| p.x > -20.0 && p.y > -20.0 && p.x < wf + 20.0 && p.y < hf + 20.0;
    let focal = camera.focal_px(h);
    let mut drawn = 0;
    for comet in &field.comets {
        let points: Vec<Option<Vector3>> = (0..=TAIL_SAMPLES)
            .map(|k| {
                let s = k as f32 / TAIL_SAMPLES as f32;
                camera.project_view(camera.world_to_view(comet.tail_point(t, star, s)), w, h)
            })
            .collect();
        if !points.iter().flatten().any(on_screen) {
            continue;
        }
        drawn += 1;
        // The tail, from its end to the nucleus: thick and bright near the
        // head, thin and faint at the end.
        for k in (0..TAIL_SAMPLES).rev() {
            let (Some(a), Some(b)) = (points[k], points[k + 1]) else {
                continue;
            };
            let s = k as f32 / TAIL_SAMPLES as f32;
            let fade = (1.0 - s) * (1.0 - s);
            let width = (0.4 * focal / a.z).clamp(0.8, 6.0) * (1.0 - 0.7 * s);
            let steps = (b.x - a.x).abs().max((b.y - a.y).abs()).ceil().max(1.0) as i32;
            for i in 0..steps {
                let f = i as f32 / steps as f32;
                let (x, y, z) = (
                    a.x + (b.x - a.x) * f,
                    a.y + (b.y - a.y) * f,
                    a.z + (b.z - a.z) * f,
                );
                glow(fb, x, y, width, z, comet.color * (0.3 * fade));
            }
        }
        if let Some(head) = points[0] {
            let r = (0.35 * focal / head.z).clamp(1.5, 7.0);
            glow(
                fb,
                head.x,
                head.y,
                r,
                head.z,
                Vector3::new(1.0, 1.0, 1.0) * 0.9,
            );
            glow(fb, head.x, head.y, 2.2 * r, head.z, comet.color * 0.35);
        }
    }
    drawn
}

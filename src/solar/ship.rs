//! The spacecraft state: position and an orthonormal `forward` / `up` basis
//! rotated with Rodrigues' formula, re-orthonormalised after every turn.
//!
//! `Laboratorio3.obj` was measured before fixing the conventions: its mesh
//! narrows towards `-Z` (nose), is widest along `X` (wings) and its top is
//! `+Y`; the bounding box is centered at `MESH_CENTER`, not at the origin.

use crate::geometry::Vertex;
use crate::pipeline::{
    DirectionalLight, NormalSource, RenderError, RenderScratch, ShadedOptions, ShadedStats,
    SurfaceSample, Uniforms, ViewProjection, intensity, render_shaded_with_scratch,
};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::shading::sphere::rotate_axis;
use raylib::math::{Matrix, Vector3};

/// Bounding-box center of `Laboratorio3.obj` (measured), subtracted so the
/// spacecraft rotates about its own middle.
pub const MESH_CENTER: Vector3 = Vector3 {
    x: 0.0,
    y: 0.255,
    z: -1.3,
};
/// Distance from `MESH_CENTER` to the farthest vertex of the mesh.
pub const MESH_RADIUS: f32 = 3.99;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipState {
    pub position: Vector3,
    /// Unit nose direction, world space.
    pub forward: Vector3,
    /// Unit top direction, world space, perpendicular to `forward`.
    pub up: Vector3,
    /// Uniform scale of the mesh.
    pub scale: f32,
    /// World units per second.
    pub move_speed: f32,
    /// Radians per second.
    pub turn_speed: f32,
    /// Light-speed boost level in `[0, 1]`, ramped by the flight controller
    /// while the space bar is held (it also drives the exhaust).
    pub boost: f32,
    /// Engine level in `[0, 1]`: how hard `W`/`A`/`S`/`D` push, eased by
    /// the flight controller (it stretches the exhaust).
    pub thrust: f32,
    /// World velocity, units per second: at rest when created, kept while
    /// coasting. Inertial flight only.
    pub velocity: Vector3,
    /// Inertial space flight (Project 3): thrust accelerates `velocity`
    /// and the ship keeps moving without input. `false` keeps the
    /// pre-project flight, where the keys move the ship directly.
    pub inertia: bool,
    /// Mouse turn still to apply, radians (inertial flight eases into it).
    pub turn: TurnState,
}

/// Mouse turn the ship has not applied yet: the mouse adds to it and the
/// flight controller eases it out (`flight::ease_turn`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TurnState {
    pub yaw: f32,
    pub pitch: f32,
}

impl ShipState {
    /// Unrotated spacecraft: nose along `-Z`, top along `+Y`.
    pub fn new(position: Vector3, scale: f32) -> Self {
        Self {
            position,
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            scale,
            move_speed: 1.5,
            turn_speed: 1.0,
            boost: 0.0,
            thrust: 0.0,
            velocity: Vector3::zero(),
            inertia: false,
            turn: TurnState::default(),
        }
    }

    /// Unit right direction (`forward x up`).
    pub fn right(&self) -> Vector3 {
        self.forward.cross(self.up).normalized()
    }

    /// Turns around `up`: positive turns the nose from `-Z` towards `-X`.
    pub fn yaw(&mut self, angle: f32) {
        self.forward = rotate_axis(self.forward, self.up, angle);
        self.renormalize();
    }

    /// Turns around `right`: positive lifts the nose.
    pub fn pitch(&mut self, angle: f32) {
        let right = self.right();
        self.forward = rotate_axis(self.forward, right, angle);
        self.up = rotate_axis(self.up, right, angle);
        self.renormalize();
    }

    /// Rotates both basis vectors around an arbitrary unit `axis`.
    pub fn rotate(&mut self, axis: Vector3, angle: f32) {
        self.forward = rotate_axis(self.forward, axis, angle);
        self.up = rotate_axis(self.up, axis, angle);
        self.renormalize();
    }

    /// Rebuilds an orthonormal basis (Gram-Schmidt) so rounding never
    /// accumulates; a degenerate basis falls back to the default one.
    pub fn renormalize(&mut self) {
        let f_len = self.forward.length();
        if !(f_len.is_finite() && f_len > 1e-6) {
            self.forward = Vector3::new(0.0, 0.0, -1.0);
        } else {
            self.forward /= f_len;
        }
        let mut right = self.forward.cross(self.up);
        if !(right.length().is_finite() && right.length() > 1e-6) {
            let alt = if self.forward.y.abs() < 0.9 {
                Vector3::new(0.0, 1.0, 0.0)
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            };
            right = self.forward.cross(alt);
        }
        let right = right.normalized();
        self.up = right.cross(self.forward).normalized();
    }

    /// Column-major model matrix: object `+X` to `right`, `+Y` to `up`,
    /// `+Z` to `-forward` (the nose is `-Z`), scaled, with `MESH_CENTER`
    /// placed at `position`.
    pub fn model_matrix(&self) -> Matrix {
        let s = self.scale;
        let (r, u, b) = (self.right(), self.up, -self.forward);
        let offset = (r * MESH_CENTER.x + u * MESH_CENTER.y + b * MESH_CENTER.z) * s;
        let t = self.position - offset;
        let mut m = Matrix::identity();
        m.m0 = r.x * s;
        m.m1 = r.y * s;
        m.m2 = r.z * s;
        m.m4 = u.x * s;
        m.m5 = u.y * s;
        m.m6 = u.z * s;
        m.m8 = b.x * s;
        m.m9 = b.y * s;
        m.m10 = b.z * s;
        m.m12 = t.x;
        m.m13 = t.y;
        m.m14 = t.z;
        m
    }

    /// An object-space point of the mesh in the world (what `model_matrix`
    /// does to it).
    pub fn to_world(&self, p: Vector3) -> Vector3 {
        let d = p - MESH_CENTER;
        self.position + (self.right() * d.x + self.up * d.y - self.forward * d.z) * self.scale
    }

    /// Rotates a world direction into the spacecraft's object space.
    pub fn to_object_dir(&self, d: Vector3) -> Vector3 {
        Vector3::new(d.dot(self.right()), d.dot(self.up), d.dot(-self.forward))
    }

    /// World radius of the scaled mesh.
    pub fn radius(&self) -> f32 {
        MESH_RADIUS * self.scale
    }
}

/// Draws the spacecraft into `framebuffer` (never cleared here) through the
/// shaded stage with flat per-face shading: the light is rotated into the
/// spacecraft's object space, so a turned ship is lit consistently with
/// the planets. Two-sided like the laboratory five interactive view.
pub fn render_ship(
    framebuffer: &mut Framebuffer,
    mesh: &[Vertex],
    ship: &ShipState,
    projection: &ViewProjection,
    light_world: Vector3,
    mode: TriangleMode,
    threads: usize,
) -> Result<ShadedStats, RenderError> {
    render_ship_scratch(
        framebuffer,
        mesh,
        ship,
        projection,
        light_world,
        mode,
        threads,
        &mut RenderScratch::new(),
    )
}

/// `render_ship` reusing the caller's buffers.
#[allow(clippy::too_many_arguments)]
pub fn render_ship_scratch(
    framebuffer: &mut Framebuffer,
    mesh: &[Vertex],
    ship: &ShipState,
    projection: &ViewProjection,
    light_world: Vector3,
    mode: TriangleMode,
    threads: usize,
    scratch: &mut RenderScratch,
) -> Result<ShadedStats, RenderError> {
    let light = ship_light(ship, light_world);
    let uniforms = Uniforms {
        model_matrix: ship.model_matrix(),
    };
    render_shaded_with_scratch(
        framebuffer,
        &uniforms,
        mesh,
        projection,
        mode,
        ship_options(threads),
        &|s, _| ship_color(&light, s),
        scratch,
    )
}

/// The world light rotated into the spacecraft's object space.
pub fn ship_light(ship: &ShipState, light_world: Vector3) -> DirectionalLight {
    DirectionalLight {
        direction: ship.to_object_dir(light_world),
        ..DirectionalLight::default()
    }
}

/// Shaded-stage options of the spacecraft: two-sided flat faces.
pub fn ship_options(threads: usize) -> ShadedOptions {
    ShadedOptions {
        cull_back_faces: false,
        normal_source: NormalSource::Face,
        threads,
    }
}

/// Flat color of one spacecraft fragment.
pub fn ship_color(light: &DirectionalLight, sample: &SurfaceSample) -> Vector3 {
    light.base_color * intensity(sample.face_normal, light)
}

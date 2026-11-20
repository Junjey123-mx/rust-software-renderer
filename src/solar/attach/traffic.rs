//! Neon traffic of the Hearts planet. The ships reuse the player's
//! spacecraft mesh (the same `Arc`, never a copy) and switch to a
//! procedural dart under `DART_PX`. They fly seeded great circles
//! (`path::Path`) in the host's frame, so they orbit and turn with it.
//! They are scenery: the player's ship neither collides with them nor
//! feels them.

use crate::geometry::{Mesh, Vertex};
use crate::pipeline::Transform;
use crate::shading::math::hex;
use crate::shading::noise::hash3;
use crate::solar::attach::path::Path;
use crate::solar::planet::HeightField;
use crate::solar::shapes::frame_matrix;
use crate::solar::ship::{MESH_CENTER, MESH_RADIUS, ShipState};
use crate::solar::{SPHERE_RADIUS, SolarMeshes};
use raylib::math::{Matrix, Vector3};
use std::f32::consts::TAU;
use std::sync::Arc;

/// Projected radius under which a neon ship is drawn as its dart, pixels.
pub const DART_PX: f32 = 30.0;
/// Size of a neon ship relative to the player's.
pub const NEON_SIZE: f32 = 0.4;
/// Altitude band of the ships over the ground, planet radii.
pub const SHIP_ALTITUDE: (f32, f32) = (0.3, 0.6);
/// Angular speed band of their routes, radians per second.
const SHIP_SPEED: (f32, f32) = (0.12, 0.3);
/// Most ships or lightcycles a planet takes.
pub const MAX_TRAFFIC: u8 = 16;
/// Hull and rim of the neon ships.
pub const NEON_BODY: Vector3 = hex(0x1A0B2E);
pub const NEON_EDGE: Vector3 = hex(0x22E6FF);
pub const NEON_FRESNEL: f32 = 2.5;
/// Seeded tries for a route clear of an excluded cone.
const ROUTE_TRIES: u32 = 32;
/// Lightcycle height over the ground, planet radii.
pub const CYCLE_ALTITUDE: f32 = 0.01;
/// Lightcycle bounding radius, planet radii.
pub const CYCLE_SIZE: f32 = 0.03;
/// Angular radius band of the cycles' loops, radians.
const CYCLE_LOOP: (f32, f32) = (0.07, 0.13);
/// Ground speed of a lightcycle, planet radii per second.
const CYCLE_SPEED: f32 = 0.18;
/// Closest a cycle's route comes to a building site, radians: the widest
/// building's half diagonal (0.037 R) plus the cycle's half width, with
/// room for the loop's curve (the cycle's ends stay on its route).
pub const CYCLE_MARGIN: f32 = 0.06;
/// Seeded tries for a lightcycle loop clear of the city.
const CYCLE_TRIES: u32 = 128;
/// Rim of the lightcycles.
pub const CYCLE_EDGE: Vector3 = hex(0xFFA62B);

/// Pushes the triangle `[a, b, c]` wound to face away from `inside`.
fn push_out(m: &mut Mesh, [a, b, c]: [Vector3; 3], inside: Vector3) {
    let i = m.vertices.len();
    m.vertices.extend([a, b, c]);
    let centroid = (a + b + c) / 3.0;
    if (b - a).cross(c - a).dot(centroid - inside) > 0.0 {
        m.indices.extend([i, i + 1, i + 2]);
    } else {
        m.indices.extend([i, i + 2, i + 1]);
    }
}

/// The far stand-in of a neon ship (34 triangles), in the spacecraft's own
/// object space so one model matrix serves both: nose to `-Z`, `+Y` up,
/// centered on `MESH_CENTER` and inside `MESH_RADIUS`. A delta hull of
/// diamond sections, a tail fin and a canopy.
pub fn dart_mesh() -> Mesh {
    let c = MESH_CENTER;
    let at = |x: f32, y: f32, z: f32| c + Vector3::new(x, y, z);
    let mut m = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    // (z, half width, half height) of each section, nose to tail.
    let sections = [(-1.8, 0.6, 0.35), (0.6, 2.0, 0.5), (2.4, 3.0, 0.25)];
    let ring =
        |(z, w, h): (f32, f32, f32)| [at(w, 0.0, z), at(0.0, h, z), at(-w, 0.0, z), at(0.0, -h, z)];
    let nose = at(0.0, 0.0, -3.6);
    let first = ring(sections[0]);
    for k in 0..4 {
        push_out(&mut m, [nose, first[k], first[(k + 1) % 4]], c);
    }
    for pair in sections.windows(2) {
        let (a, b) = (ring(pair[0]), ring(pair[1]));
        for k in 0..4 {
            let n = (k + 1) % 4;
            push_out(&mut m, [a[k], b[k], b[n]], c);
            push_out(&mut m, [a[k], b[n], a[n]], c);
        }
    }
    let tail = ring(sections[2]);
    push_out(&mut m, [tail[0], tail[1], tail[2]], at(0.0, 0.0, 0.0));
    push_out(&mut m, [tail[0], tail[2], tail[3]], at(0.0, 0.0, 0.0));
    // Tail fin: a thin triangular prism over the back of the hull.
    let fin = |x: f32| [at(x, 0.45, 0.9), at(x, 1.6, 2.4), at(x, 0.25, 2.4)];
    let (l, r) = (fin(-0.08), fin(0.08));
    let fin_center = at(0.0, 0.8, 1.9);
    push_out(&mut m, l, fin_center);
    push_out(&mut m, r, fin_center);
    for k in 0..3 {
        let n = (k + 1) % 3;
        push_out(&mut m, [l[k], r[k], r[n]], fin_center);
        push_out(&mut m, [l[k], r[n], l[n]], fin_center);
    }
    // Canopy: a low four-sided pyramid on the hull.
    let top = at(0.0, 0.75, -0.4);
    let base = [
        at(0.35, 0.3, -1.0),
        at(0.35, 0.4, 0.2),
        at(-0.35, 0.4, 0.2),
        at(-0.35, 0.3, -1.0),
    ];
    for k in 0..4 {
        push_out(
            &mut m,
            [top, base[k], base[(k + 1) % 4]],
            at(0.0, 0.3, -0.4),
        );
    }
    m
}

/// One neon ship: the player's spacecraft mesh and the shared dart, at
/// `NEON_SIZE` of the player's size.
#[derive(Debug, Clone)]
pub struct NeonShip {
    pub mesh: Arc<Vec<Vertex>>,
    pub dart: Arc<Vec<Vertex>>,
    /// Model scale: its radius over `MESH_RADIUS`.
    pub scale: f32,
}

impl NeonShip {
    /// A ship of the player's meshes, `radius` world units across its
    /// bounding sphere.
    pub fn new(meshes: &SolarMeshes, radius: f32) -> Self {
        Self {
            mesh: Arc::clone(&meshes.ship),
            dart: Arc::clone(&meshes.dart),
            scale: radius / MESH_RADIUS,
        }
    }

    /// Bounding radius, world units.
    pub fn radius(&self) -> f32 {
        MESH_RADIUS * self.scale
    }

    /// The mesh to draw at `radius_px` pixels of projected radius.
    pub fn lod(&self, radius_px: f32) -> &Arc<Vec<Vertex>> {
        if radius_px < DART_PX {
            &self.dart
        } else {
            &self.mesh
        }
    }

    /// Model matrix with its center at `position`, nose along `forward`
    /// and top along `up` (the spacecraft's convention, `ShipState`).
    pub fn matrix(&self, position: Vector3, forward: Vector3, up: Vector3) -> Matrix {
        let mut s = ShipState::new(position, self.scale);
        s.forward = forward;
        s.up = up;
        s.renormalize();
        s.model_matrix()
    }
}

/// Dark hull with a neon rim: the edge color grows where the surface turns
/// away from the eye (Fresnel), over a dim lit body. `light` and `view` are
/// unit directions in the ship's object space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeonShader {
    pub body: Vector3,
    pub edge: Vector3,
    pub light: Vector3,
    pub view: Vector3,
    /// Rim exponent (higher: a thinner rim).
    pub fresnel: f32,
}

impl NeonShader {
    pub fn shade(&self, face_normal: Vector3) -> Vector3 {
        let lit = face_normal.dot(self.light).max(0.0);
        let rim = (1.0 - face_normal.dot(self.view).abs())
            .clamp(0.0, 1.0)
            .powf(self.fresnel.max(0.1));
        self.body * (0.15 + 0.85 * lit) + self.edge * rim
    }
}

/// A value in `[0, 1)` from the seed and two indices.
fn unit_hash(seed: u32, i: u32, j: i32) -> f32 {
    (hash3(i as i32, j, 0x7aff, seed) >> 8) as f32 / (1u32 << 24) as f32
}

fn lerp((a, b): (f32, f32), x: f32) -> f32 {
    a + (b - a) * x
}

/// True when no point of `path` (128 samples of one lap) comes within
/// `margin` radians of a site (unit directions).
pub fn route_clear_of_buildings(path: &Path, sites: &[Vector3], margin: f32) -> bool {
    let (period, cos) = (path.period(), margin.cos());
    (0..128).all(|k| {
        let d = path.direction(period * k as f32 / 128.0);
        sites.iter().all(|s| s.dot(d) < cos)
    })
}

/// A uniform unit direction from two values in `[0, 1)`.
fn sphere_point(a: f32, b: f32) -> Vector3 {
    let z = 2.0 * a - 1.0;
    let (s, c) = (TAU * b).sin_cos();
    let r = (1.0 - z * z).max(0.0).sqrt();
    Vector3::new(r * c, z, r * s)
}

/// The traffic of one planet: its ships and lightcycles on their routes
/// over `field`.
#[derive(Debug, Clone)]
pub struct Traffic {
    pub ships: Vec<(Path, NeonShip)>,
    /// Lightcycle loops on the ground (`with_cycles`).
    pub cycles: Vec<Path>,
    /// The shared lightcycle mesh (`SolarMeshes::lightcycle`).
    pub cycle_mesh: Arc<Vec<Vertex>>,
    /// The host's ground (`Flat` for a sphere).
    pub field: HeightField,
    /// Farthest reach from the host center, host radii.
    pub reach: f32,
}

impl Traffic {
    /// `ships` neon ships of `ship_radius` world units over a host of
    /// `host_radius`: great circles of seeded axes, speeds, directions,
    /// phases and altitudes (`SHIP_ALTITUDE`). With `exclude` (a unit
    /// direction of the host and an angle) a route that passes within the
    /// angle of it is drawn again from the next hashes; one that never
    /// clears it after `ROUTE_TRIES` is left out.
    pub fn new(
        meshes: &SolarMeshes,
        field: HeightField,
        ships: u8,
        seed: u32,
        ship_radius: f32,
        host_radius: f32,
        exclude: Option<(Vector3, f32)>,
    ) -> Self {
        let ships = (0..u32::from(ships.min(MAX_TRAFFIC)))
            .filter_map(|i| {
                (0..ROUTE_TRIES).find_map(|k| {
                    let h = |j: i32| unit_hash(seed, i, j + 8 * k as i32);
                    let axis = sphere_point(h(0), h(1));
                    // Angular distance from the cone's center to the
                    // circle: |asin(axis . dir)|.
                    if let Some((dir, angle)) = exclude
                        && axis.dot(dir.normalized()).abs() < angle.sin()
                    {
                        return None;
                    }
                    let speed = lerp(SHIP_SPEED, h(2)) * if h(3) < 0.5 { -1.0 } else { 1.0 };
                    let path = Path::GreatCircle {
                        axis,
                        speed,
                        phase: TAU * h(4),
                        altitude: lerp(SHIP_ALTITUDE, h(5)),
                    };
                    Some((path, NeonShip::new(meshes, ship_radius)))
                })
            })
            .collect();
        Self {
            ships,
            cycles: Vec::new(),
            cycle_mesh: Arc::clone(&meshes.lightcycle),
            field,
            reach: 1.0 + field.max_height() + SHIP_ALTITUDE.1 + ship_radius / host_radius,
        }
    }

    /// Adds `cycles` lightcycles on seeded loops (`Path::Ring`,
    /// `CYCLE_ALTITUDE` over the ground) that keep `CYCLE_MARGIN` from
    /// every site and stay out of the `exclude` cone. A cycle whose
    /// `CYCLE_TRIES` loops all fail is left out, with a warning on stderr.
    pub fn with_cycles(
        mut self,
        cycles: u8,
        seed: u32,
        sites: &[Vector3],
        exclude: Option<(Vector3, f32)>,
    ) -> Self {
        for i in 0..u32::from(cycles.min(MAX_TRAFFIC)) {
            let route = (0..CYCLE_TRIES).find_map(|k| {
                let h = |j: i32| unit_hash(seed ^ 0xc1c1e, i, j + 8 * k as i32);
                let axis = sphere_point(h(0), h(1));
                let radius = lerp(CYCLE_LOOP, h(2));
                if let Some((dir, angle)) = exclude
                    && axis.dot(dir.normalized()).clamp(-1.0, 1.0).acos() < angle + radius
                {
                    return None;
                }
                let sign = if h(3) < 0.5 { -1.0 } else { 1.0 };
                let path = Path::Ring {
                    axis,
                    radius,
                    speed: sign * CYCLE_SPEED / radius.sin(),
                    phase: TAU * h(4),
                    altitude: CYCLE_ALTITUDE,
                };
                route_clear_of_buildings(&path, sites, CYCLE_MARGIN).then_some(path)
            });
            match route {
                Some(path) => self.cycles.push(path),
                None => eprintln!(
                    "warning: lightcycle {i} left out: no loop clear of the city in {CYCLE_TRIES} tries"
                ),
            }
        }
        if !self.cycles.is_empty() {
            let top = 1.0 + self.field.max_height() + CYCLE_ALTITUDE + 2.0 * CYCLE_SIZE;
            self.reach = self.reach.max(top);
        }
        self
    }

    /// Center, heading and top of `path` at time `t`, world space, on a
    /// host placed by `host`.
    fn place(&self, path: &Path, t: f32, host: &Transform) -> (Vector3, Vector3, Vector3) {
        let p = path.pose(t, &self.field, SPHERE_RADIUS);
        (
            host.position + host.to_world_dir(p.pos) * host.scale,
            host.to_world_dir(p.tangent).normalized(),
            host.to_world_dir(p.up).normalized(),
        )
    }

    /// Center, nose and top of ship `i` at time `t`, world space, on a host
    /// placed by `host`.
    pub fn world_pose(&self, i: usize, t: f32, host: &Transform) -> (Vector3, Vector3, Vector3) {
        self.place(&self.ships[i].0, t, host)
    }

    /// Ground point under lightcycle `i` (plus its altitude), heading and
    /// top at time `t`, world space.
    pub fn cycle_pose(&self, i: usize, t: f32, host: &Transform) -> (Vector3, Vector3, Vector3) {
        self.place(&self.cycles[i], t, host)
    }

    /// World bounding radius of a lightcycle on `host`.
    pub fn cycle_radius(&self, host: &Transform) -> f32 {
        CYCLE_SIZE * SPHERE_RADIUS * host.scale
    }

    /// Model matrix of lightcycle `i` at time `t`: the mesh's base on its
    /// pose, nose (`-Z`) along the heading, `+Y` up.
    pub fn cycle_matrix(&self, i: usize, t: f32, host: &Transform) -> Matrix {
        let (base, forward, up) = self.cycle_pose(i, t, host);
        let k = self.cycle_radius(host);
        let right = forward.cross(up).normalized();
        frame_matrix(right * k, up * k, -forward * k, base)
    }
}

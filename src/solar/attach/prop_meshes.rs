//! Low-poly surface models of the relief planets, built once as indexed
//! meshes with their base at the origin and "up" along +Y: a clover tree
//! (hexagonal trunk and stacked cone canopies), a faceted boulder, the
//! Hearts city's stepped neon buildings and its lightcycle.

use crate::geometry::Mesh;
use crate::shading::noise::hash3;
use crate::solar::planet::icosphere::icosphere;
use crate::solar::shapes::{End, loft, merge};
use raylib::math::Vector3;

/// Tree variants: young, mature and tall.
pub const TREE_VARIANTS: u8 = 3;

/// Regular `sides`-gon of `radius` at height `y`, turned by `turn`.
fn ring(sides: usize, radius: f32, y: f32, turn: f32) -> Vec<Vector3> {
    (0..sides)
        .map(|k| {
            let a = turn + k as f32 / sides as f32 * std::f32::consts::TAU;
            Vector3::new(radius * a.cos(), y, radius * a.sin())
        })
        .collect()
}

/// Trunk and canopy of tree `variant` (out of range: variant 0), each a
/// closed solid. Base at the origin, about one unit tall.
pub fn tree_parts(variant: u8) -> [Mesh; 2] {
    // (trunk height, trunk radius, cones as (base y, base radius, apex y))
    let (trunk_h, trunk_r, cones): (f32, f32, &[(f32, f32, f32)]) = match variant {
        1 => (
            0.3,
            0.07,
            &[(0.22, 0.36, 0.62), (0.44, 0.28, 0.82), (0.64, 0.19, 1.0)],
        ),
        2 => (
            0.38,
            0.06,
            &[(0.3, 0.26, 0.7), (0.52, 0.2, 0.9), (0.72, 0.13, 1.08)],
        ),
        _ => (0.24, 0.06, &[(0.18, 0.3, 0.58), (0.4, 0.21, 0.8)]),
    };
    let trunk = loft(
        &[
            ring(6, trunk_r, 0.0, 0.0),
            ring(6, trunk_r * 0.8, trunk_h, 0.0),
        ],
        End::Cap,
        End::Cap,
        Vector3::new(0.0, trunk_h * 0.5, 0.0),
    );
    let canopy: Vec<Mesh> = cones
        .iter()
        .enumerate()
        .map(|(k, &(y, r, apex))| {
            loft(
                &[ring(7, r, y, k as f32 * 0.45)],
                End::Cap,
                End::Apex(Vector3::new(0.0, apex, 0.0)),
                Vector3::new(0.0, y + (apex - y) * 0.25, 0.0),
            )
        })
        .collect();
    [trunk, merge(&canopy)]
}

/// Tree `variant` as one mesh (trunk and canopy); out of range: variant 0.
pub fn tree_mesh(variant: u8) -> Mesh {
    merge(&tree_parts(variant))
}

/// Boulder of `seed` (any seed, 0 included, is a valid rock): the level 1
/// icosphere (80 triangles) with each vertex pushed in or out by a hash of
/// the seed, flattened to 60 % of its height and sunk 10 % into the ground.
pub fn rock_mesh(seed: u32) -> Mesh {
    let ico = icosphere(1).expect("level 1 is in range");
    let mut vertices: Vec<Vector3> = ico
        .positions
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let h = (hash3(i as i32, 0x0c, 0x5e, seed) >> 8) as f32 / (1u32 << 24) as f32;
            let p = p * (0.78 + 0.36 * h);
            Vector3::new(p.x, p.y * 0.6, p.z)
        })
        .collect();
    let bottom = vertices.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let height = vertices.iter().map(|p| p.y).fold(f32::MIN, f32::max) - bottom;
    for p in &mut vertices {
        p.y -= bottom + 0.1 * height;
    }
    let indices = ico
        .indices
        .iter()
        .flat_map(|t| t.map(|i| i as usize))
        .collect();
    Mesh::new(vertices, indices).expect("the icosphere indices are valid")
}

/// Building variants of the Hearts city: stepped tower, civic block and
/// needle tower.
pub const TOWER_VARIANTS: u8 = 3;
/// Width of a neon edge strip, model units.
pub const STRIP_WIDTH: f32 = 0.02;
/// How far a strip stands off its face, model units (no z-fighting).
pub const STRIP_LIFT: f32 = 0.006;

/// Rectangle of half sizes `w` x `d` at height `y`, counterclockwise from
/// above.
fn rectangle(w: f32, d: f32, y: f32) -> [Vector3; 4] {
    [
        Vector3::new(-w, y, -d),
        Vector3::new(-w, y, d),
        Vector3::new(w, y, d),
        Vector3::new(w, y, -d),
    ]
}

/// One quad wound to face `outward`.
fn quad(p: [Vector3; 4], outward: Vector3) -> Mesh {
    let facing = (p[1] - p[0]).cross(p[2] - p[0]).dot(outward) > 0.0;
    let indices = if facing {
        vec![0, 1, 2, 0, 2, 3]
    } else {
        vec![0, 2, 1, 0, 3, 2]
    };
    Mesh {
        vertices: p.to_vec(),
        indices,
    }
}

/// Building `variant` of the Hearts city (out of range: variant 0): the
/// structure, a stack of closed boxes (2 or 3 tiers) that narrow upwards,
/// and its neon edges, thin strips along every vertical corner and every
/// top edge, `STRIP_LIFT` off the faces and wound outwards. The edges are
/// their own mesh because baking loses the model's local coordinates, so
/// the shader could not find them. Base at the origin, about one unit
/// tall (the civic block is lower and wider).
pub fn tower_mesh(variant: u8) -> (Mesh, Mesh) {
    // (half width, half depth, height) of each tier, bottom up.
    let tiers: &[(f32, f32, f32)] = match variant {
        1 => &[(0.42, 0.3, 0.28), (0.26, 0.18, 0.18)],
        2 => &[(0.14, 0.14, 0.75), (0.09, 0.09, 0.3), (0.04, 0.04, 0.2)],
        _ => &[(0.3, 0.3, 0.5), (0.21, 0.21, 0.32), (0.12, 0.12, 0.22)],
    };
    stepped(tiers)
}

/// The monumental pyramid of the Hearts planet: `levels` square steps
/// (at least 2; fewer count as 2) from a base 1 unit wide to a flat top
/// 0.16 wide, 1 unit tall, with neon strips on every step's edges like
/// the buildings. Base at the origin.
pub fn pyramid_mesh(levels: u8) -> (Mesh, Mesh) {
    let levels = levels.max(2) as usize;
    let tiers: Vec<(f32, f32, f32)> = (0..levels)
        .map(|k| {
            let w = 0.5 - 0.42 * k as f32 / (levels - 1) as f32;
            (w, w, 1.0 / levels as f32)
        })
        .collect();
    stepped(&tiers)
}

/// Stacked closed boxes `(half width, half depth, height)`, bottom up,
/// with their neon edge strips.
fn stepped(tiers: &[(f32, f32, f32)]) -> (Mesh, Mesh) {
    let up = Vector3::new(0.0, 1.0, 0.0);
    let (mut blocks, mut strips, mut y) = (Vec::new(), Vec::new(), 0.0);
    for &(w, d, h) in tiers {
        let (lo, hi) = (rectangle(w, d, y), rectangle(w, d, y + h));
        blocks.push(loft(
            &[lo.to_vec(), hi.to_vec()],
            End::Cap,
            End::Cap,
            Vector3::new(0.0, y + 0.5 * h, 0.0),
        ));
        for k in 0..4 {
            // Vertical corner: a strip facing the corner's diagonal.
            let corner = lo[k];
            let out = Vector3::new(corner.x, 0.0, corner.z).normalized();
            let side = Vector3::new(-out.z, 0.0, out.x) * (0.5 * STRIP_WIDTH);
            let foot = corner + out * STRIP_LIFT;
            strips.push(quad(
                [
                    foot - side,
                    foot + side,
                    foot + side + up * h,
                    foot - side + up * h,
                ],
                out,
            ));
            // Top edge: a strip on the face, just under the edge.
            let (a, b) = (hi[k], hi[(k + 1) % 4]);
            let mid = (a + b) * 0.5;
            let normal = Vector3::new(mid.x, 0.0, mid.z).normalized();
            let (a, b) = (a + normal * STRIP_LIFT, b + normal * STRIP_LIFT);
            let drop = up * STRIP_WIDTH;
            strips.push(quad([a - drop, b - drop, b, a], normal));
        }
        y += h;
    }
    (merge(&blocks), merge(&strips))
}

/// Triangle `a b c` wound to face `outward`, appended to `indices`.
fn facing(vertices: &[Vector3], indices: &mut Vec<usize>, [a, b, c]: [usize; 3], outward: Vector3) {
    let normal = (vertices[b] - vertices[a]).cross(vertices[c] - vertices[a]);
    if normal.dot(outward) >= 0.0 {
        indices.extend([a, b, c]);
    } else {
        indices.extend([a, c, b]);
    }
}

/// Outline of the heart disc: `x = 16 sin^3 t`, `y = 13 cos t - 5 cos 2t -
/// 2 cos 3t - cos 4t`, scaled to 1 unit wide and centered on its middle.
pub fn heart_outline(samples: usize) -> Vec<(f32, f32)> {
    (0..samples)
        .map(|k| {
            let t = k as f32 / samples as f32 * std::f32::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            (x / 32.0, (y + 2.5) / 32.0)
        })
        .collect()
}

/// The Hearts planet's identity disc: the parametric heart (`samples`
/// points of its outline, at least 8) extruded 0.18 units deep with both
/// faces fanned from the centroid, and a thin halo ring around it (a torus
/// of 10 x 4 quads). Centered at the origin, the heart in the XY plane, +Y
/// up, about one unit wide: 4 * samples + 80 triangles.
pub fn heart_disc_mesh(samples: u32) -> Result<Mesh, String> {
    if samples < 8 {
        return Err(format!("the heart needs at least 8 samples, got {samples}"));
    }
    let n = samples as usize;
    let outline = heart_outline(n);
    let half = 0.09;
    let mut vertices: Vec<Vector3> = Vec::with_capacity(2 * n + 2 + 40);
    for z in [half, -half] {
        vertices.extend(outline.iter().map(|&(x, y)| Vector3::new(x, y, z)));
    }
    let (cx, cy) = outline
        .iter()
        .fold((0.0, 0.0), |(a, b), &(x, y)| (a + x, b + y));
    let (cx, cy) = (cx / n as f32, cy / n as f32);
    vertices.push(Vector3::new(cx, cy, half));
    vertices.push(Vector3::new(cx, cy, -half));
    let (front, back) = (2 * n, 2 * n + 1);
    let mut indices = Vec::new();
    let z = Vector3::new(0.0, 0.0, 1.0);
    for k in 0..n {
        let k1 = (k + 1) % n;
        facing(&vertices, &mut indices, [front, k, k1], z);
        facing(&vertices, &mut indices, [back, n + k, n + k1], -z);
        // The side quad faces away from the centroid.
        let mid = (vertices[k] + vertices[k1]) * 0.5;
        let out = Vector3::new(mid.x - cx, mid.y - cy, 0.0);
        facing(&vertices, &mut indices, [k, k1, n + k1], out);
        facing(&vertices, &mut indices, [k, n + k1, n + k], out);
    }
    // Halo: a torus around the heart, in its plane.
    let (major, minor, segments, sides) = (0.66, 0.025, 10, 4);
    let base = vertices.len();
    for i in 0..segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let center = Vector3::new(cx + major * a.cos(), cy + major * a.sin(), 0.0);
        let radial = Vector3::new(a.cos(), a.sin(), 0.0);
        for j in 0..sides {
            let b = j as f32 / sides as f32 * std::f32::consts::TAU;
            vertices.push(center + (radial * b.cos() + z * b.sin()) * minor);
        }
    }
    for i in 0..segments {
        for j in 0..sides {
            let at = |i: usize, j: usize| base + (i % segments) * sides + j % sides;
            let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            let ring = Vector3::new(cx, cy, 0.0);
            let center = (vertices[a] + vertices[b] + vertices[c] + vertices[d]) * 0.25;
            let tube = {
                let ang = (i as f32 + 0.5) / segments as f32 * std::f32::consts::TAU;
                ring + Vector3::new(ang.cos(), ang.sin(), 0.0) * major
            };
            let out = center - tube;
            facing(&vertices, &mut indices, [a, b, c], out);
            facing(&vertices, &mut indices, [a, c, d], out);
        }
    }
    Ok(Mesh { vertices, indices })
}

/// Nose and top of the lightcycle model: the spacecraft's convention
/// (`Laboratorio3.obj` flies nose first to `-Z`, `+Y` up).
pub const LIGHTCYCLE_FORWARD: Vector3 = Vector3::new(0.0, 0.0, -1.0);
pub const LIGHTCYCLE_UP: Vector3 = Vector3::new(0.0, 1.0, 0.0);

/// `mesh` ready to place on a surface: an error when it has no triangles
/// or a degenerate one (zero area); otherwise moved so its footprint is
/// centered on the origin with its lowest point at `y = 0`, and scaled so
/// its bounding sphere (about its box center) has radius 1.
pub fn normalize_model(mut mesh: Mesh) -> Result<Mesh, String> {
    if mesh.indices.len() < 3 || !mesh.indices.len().is_multiple_of(3) {
        return Err("the model has no triangles".into());
    }
    for t in mesh.indices.chunks(3) {
        let p = t
            .iter()
            .map(|&i| mesh.vertices.get(i))
            .collect::<Option<Vec<_>>>();
        let p = p.ok_or("a face points past the vertices")?;
        let area2 = (*p[1] - *p[0]).cross(*p[2] - *p[0]).length();
        if !area2.is_finite() || area2 <= 1e-9 {
            return Err("the model has a degenerate face".into());
        }
    }
    let (lo, hi) = mesh.vertices.iter().fold(
        (Vector3::one() * f32::MAX, Vector3::one() * f32::MIN),
        |(lo, hi), p| {
            (
                Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        },
    );
    let center = (lo + hi) * 0.5;
    let radius = mesh
        .vertices
        .iter()
        .map(|p| (*p - center).length())
        .fold(0.0, f32::max);
    let foot = Vector3::new(center.x, lo.y, center.z);
    for p in &mut mesh.vertices {
        *p = (*p - foot) / radius;
    }
    Ok(mesh)
}

/// An original low-poly lightcycle (82 triangles), `normalize_model`ed:
/// two octagonal wheels under a wedge hull that ends in a pointed nose
/// (`LIGHTCYCLE_FORWARD`) and a flat tail, with a low canopy over the rider
/// position.
pub fn lightcycle_mesh() -> Result<Mesh, String> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    // Wheels: octagonal prisms 0.25 wide, radius 0.45, axles at z = -0.8
    // (front) and 0.8 (rear).
    for z0 in [-0.8f32, 0.8] {
        let axle = Vector3::new(0.0, 0.45, z0);
        let base = vertices.len();
        for x in [-0.125f32, 0.125] {
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                vertices.push(axle + Vector3::new(x, 0.45 * a.sin(), 0.45 * a.cos()));
            }
        }
        for k in 0..8 {
            let n = (k + 1) % 8;
            let (a, b, c, d) = (base + k, base + n, base + 8 + n, base + 8 + k);
            let out = (vertices[a] + vertices[c]) * 0.5 - axle;
            let out = Vector3::new(0.0, out.y, out.z);
            facing(&vertices, &mut indices, [a, b, c], out);
            facing(&vertices, &mut indices, [a, c, d], out);
        }
        for (side, x) in [(0, -1.0f32), (8, 1.0)] {
            for k in 1..7 {
                let tri = [base + side, base + side + k, base + side + k + 1];
                facing(&vertices, &mut indices, tri, Vector3::new(x, 0.0, 0.0));
            }
        }
    }
    // Hull: box sections (z, half width, bottom, top) from the nose tip to
    // the flat tail.
    let sections = [
        (-0.5f32, 0.22f32, 0.35f32, 0.95f32),
        (0.5, 0.22, 0.35, 0.85),
        // Past the rear wheel (z 1.25), so the tail is the rearmost part.
        (1.38, 0.1, 0.45, 0.65),
    ];
    let hull_center = Vector3::new(0.0, 0.65, 0.2);
    let ring = |(z, w, yb, yt): (f32, f32, f32, f32)| {
        [
            Vector3::new(w, yb, z),
            Vector3::new(w, yt, z),
            Vector3::new(-w, yt, z),
            Vector3::new(-w, yb, z),
        ]
    };
    let tip = vertices.len();
    vertices.push(Vector3::new(0.0, 0.62, -1.35));
    let mut rings = Vec::new();
    for s in sections {
        rings.push(vertices.len());
        vertices.extend(ring(s));
    }
    let out = |vs: &[Vector3], tri: [usize; 3]| {
        (vs[tri[0]] + vs[tri[1]] + vs[tri[2]]) / 3.0 - hull_center
    };
    for k in 0..4 {
        let tri = [tip, rings[0] + k, rings[0] + (k + 1) % 4];
        let o = out(&vertices, tri);
        facing(&vertices, &mut indices, tri, o);
    }
    for pair in rings.windows(2) {
        for k in 0..4 {
            let n = (k + 1) % 4;
            for tri in [
                [pair[0] + k, pair[1] + k, pair[1] + n],
                [pair[0] + k, pair[1] + n, pair[0] + n],
            ] {
                let o = out(&vertices, tri);
                facing(&vertices, &mut indices, tri, o);
            }
        }
    }
    let tail = rings[2];
    for tri in [[tail, tail + 1, tail + 2], [tail, tail + 2, tail + 3]] {
        facing(&vertices, &mut indices, tri, Vector3::new(0.0, 0.0, 1.0));
    }
    // Canopy: a low four-sided pyramid on the hull's top.
    let apex = vertices.len();
    vertices.push(Vector3::new(0.0, 1.12, -0.05));
    let rim = vertices.len();
    vertices.extend([
        Vector3::new(0.16, 0.92, -0.35),
        Vector3::new(0.16, 0.88, 0.3),
        Vector3::new(-0.16, 0.88, 0.3),
        Vector3::new(-0.16, 0.92, -0.35),
    ]);
    for k in 0..4 {
        let tri = [apex, rim + k, rim + (k + 1) % 4];
        let o = out(&vertices, tri) + Vector3::new(0.0, 1.0, 0.0);
        facing(&vertices, &mut indices, tri, o);
    }
    normalize_model(Mesh { vertices, indices })
}

//! Low-poly surface models of the relief planets, built once as indexed
//! meshes with their base at the origin and "up" along +Y: a clover tree
//! (hexagonal trunk and stacked cone canopies), a faceted boulder and the
//! Hearts city's stepped neon buildings.

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

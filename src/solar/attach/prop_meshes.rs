//! Low-poly surface models of the relief planets, built once as indexed
//! meshes with their base at the origin and "up" along +Y: a clover tree
//! (hexagonal trunk and stacked cone canopies) and a faceted boulder.

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

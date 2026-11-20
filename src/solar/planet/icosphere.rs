//! Subdivided icosahedron: an evenly tessellated unit sphere with shared
//! vertices, the base of the planets' relief and levels of detail
//! (`sphere.obj` stays as it is for the pre-project bodies).

use crate::geometry::Vertex;
use raylib::math::Vector3;
use std::collections::HashMap;

/// Highest subdivision level (20 480 triangles).
pub const MAX_LEVEL: u8 = 5;

/// Indexed triangle mesh on the unit sphere, counter-clockwise seen from
/// outside (the winding of `sphere.obj`).
#[derive(Debug, Clone, PartialEq)]
pub struct IcoMesh {
    /// Unit directions.
    pub positions: Vec<Vector3>,
    pub indices: Vec<[u32; 3]>,
}

/// The regular icosahedron (golden ratio), 12 vertices and 20 faces.
pub fn icosahedron() -> IcoMesh {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let positions = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| Vector3::new(x, y, z).normalized())
    .collect();
    let indices = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    IcoMesh { positions, indices }
}

/// Every triangle split in four at its edge midpoints, pushed back onto the
/// sphere. A midpoint is made once per edge (keyed by its sorted ends), so
/// neighbors share it and no crack can open.
pub fn subdivide(m: &IcoMesh) -> IcoMesh {
    let mut positions = m.positions.clone();
    let mut midpoints: HashMap<(u32, u32), u32> = HashMap::new();
    let mut midpoint = |a: u32, b: u32, positions: &mut Vec<Vector3>| {
        *midpoints.entry((a.min(b), a.max(b))).or_insert_with(|| {
            let p = ((positions[a as usize] + positions[b as usize]) * 0.5).normalized();
            positions.push(p);
            (positions.len() - 1) as u32
        })
    };
    let mut indices = Vec::with_capacity(m.indices.len() * 4);
    for &[a, b, c] in &m.indices {
        let ab = midpoint(a, b, &mut positions);
        let bc = midpoint(b, c, &mut positions);
        let ca = midpoint(c, a, &mut positions);
        indices.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
    }
    IcoMesh { positions, indices }
}

/// The icosahedron subdivided `level` times: `20 * 4^level` triangles. A
/// level above `MAX_LEVEL` is an error (memory grows fourfold per level).
pub fn icosphere(level: u8) -> Result<IcoMesh, String> {
    if level > MAX_LEVEL {
        return Err(format!(
            "icosphere level {level} is above the maximum {MAX_LEVEL}"
        ));
    }
    Ok((0..level).fold(icosahedron(), |m, _| subdivide(&m)))
}

/// The pipeline's mesh (three corners per triangle, white): each unit
/// direction `n` placed at `n * radius_of(n)`, winding kept.
pub fn to_mesh(m: &IcoMesh, radius_of: impl Fn(Vector3) -> f32) -> Vec<Vertex> {
    let placed: Vec<Vector3> = m.positions.iter().map(|&n| n * radius_of(n)).collect();
    m.indices
        .iter()
        .flat_map(|t| t.map(|i| Vertex::new_with_color(placed[i as usize], Vector3::one())))
        .collect()
}

//! Low-poly building blocks of the solar scene's procedural geometry (the
//! royal crown, the playing cards, the thruster flames): convex lofts of
//! polygon rings, built once as indexed meshes and expanded for drawing.

use crate::geometry::Mesh;
use raylib::math::Vector3;

/// How a loft ends before its first or after its last ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum End {
    /// Left open (for an end buried inside another solid).
    Open,
    /// Flat polygon over the ring.
    Cap,
    /// Fan of triangles meeting at a point.
    Apex(Vector3),
}

/// Convex solid through `rings` (each the same number of corners, in
/// order around the solid) with the given ends. Every triangle is wound so
/// its normal points away from `interior`, a point strictly inside the
/// hull, so back-face culling keeps the outside.
pub fn loft(rings: &[Vec<Vector3>], start: End, end: End, interior: Vector3) -> Mesh {
    let n = rings[0].len();
    let mut vertices: Vec<Vector3> = rings.iter().flatten().copied().collect();
    let mut indices = Vec::new();
    let mut tri = |vertices: &[Vector3], a: usize, b: usize, c: usize| {
        let (pa, pb, pc) = (vertices[a], vertices[b], vertices[c]);
        let normal = (pb - pa).cross(pc - pa);
        let outward = normal.dot((pa + pb + pc) / 3.0 - interior) >= 0.0;
        indices.extend(if outward { [a, b, c] } else { [a, c, b] });
    };
    for r in 0..rings.len() - 1 {
        for j in 0..n {
            let k = (j + 1) % n;
            let (a, b, c, d) = (r * n + j, r * n + k, (r + 1) * n + k, (r + 1) * n + j);
            tri(&vertices, a, b, c);
            tri(&vertices, a, c, d);
        }
    }
    for (ring, cap) in [(0, start), ((rings.len() - 1) * n, end)] {
        match cap {
            End::Open => {}
            End::Cap => {
                for j in 1..n - 1 {
                    tri(&vertices, ring, ring + j, ring + j + 1);
                }
            }
            End::Apex(p) => {
                vertices.push(p);
                let top = vertices.len() - 1;
                for j in 0..n {
                    tri(&vertices, ring + j, ring + (j + 1) % n, top);
                }
            }
        }
    }
    Mesh::new(vertices, indices).expect("a loft has at least one triangle")
}

/// One mesh holding every part, indices shifted to the merged positions.
pub fn merge(parts: &[Mesh]) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for part in parts {
        let base = vertices.len();
        vertices.extend_from_slice(&part.vertices);
        indices.extend(part.indices.iter().map(|i| i + base));
    }
    Mesh::new(vertices, indices).expect("merged parts keep valid indices")
}

/// Positions multiplied by `factor` (uniform scale about the origin).
pub fn scaled(mesh: &Mesh, factor: f32) -> Mesh {
    Mesh {
        vertices: mesh.vertices.iter().map(|&p| p * factor).collect(),
        indices: mesh.indices.clone(),
    }
}

/// Octahedron centered at `center` with half-axes `a`, `b` (its girdle) and
/// `c` (its two tips): the faceted gems and finials.
pub fn octahedron(center: Vector3, a: Vector3, b: Vector3, c: Vector3) -> Mesh {
    loft(
        &[vec![center + a, center + b, center - a, center - b]],
        End::Apex(center - c),
        End::Apex(center + c),
        center,
    )
}

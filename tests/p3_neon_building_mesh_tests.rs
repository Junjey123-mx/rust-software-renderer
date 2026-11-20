//! Project 3 Hearts buildings: stepped closed boxes wound outwards, with
//! neon strips that lie on their edges, unlike the clover's trees and
//! rocks.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::solar::attach::prop_meshes::{
    STRIP_LIFT, STRIP_WIDTH, TOWER_VARIANTS, rock_mesh, tower_mesh, tree_mesh,
};
use std::collections::HashMap;

fn triangles(m: &Mesh) -> impl Iterator<Item = [Vector3; 3]> + '_ {
    m.indices
        .chunks(3)
        .map(|t| [m.vertices[t[0]], m.vertices[t[1]], m.vertices[t[2]]])
}

/// Width, height and depth of the bounding box.
fn size(m: &Mesh) -> Vector3 {
    let (mut lo, mut hi) = (Vector3::one() * f32::MAX, Vector3::one() * f32::MIN);
    for v in &m.vertices {
        lo = Vector3::new(lo.x.min(v.x), lo.y.min(v.y), lo.z.min(v.z));
        hi = Vector3::new(hi.x.max(v.x), hi.y.max(v.y), hi.z.max(v.z));
    }
    hi - lo
}

#[test]
fn distinct_from_tree_and_rock() {
    let tree = tree_mesh(0);
    let rock = rock_mesh(1);
    for v in 0..TOWER_VARIANTS {
        let (body, edges) = tower_mesh(v);
        let n = body.indices.len() / 3;
        assert_ne!(n, tree.indices.len() / 3);
        assert_ne!(n, rock.indices.len() / 3);
        // Boxes: every face is horizontal or vertical, which no tree cone
        // or rock facet is.
        for [a, b, c] in triangles(&body) {
            let normal = (b - a).cross(c - a).normalized();
            let axis = normal.x.abs().max(normal.y.abs()).max(normal.z.abs());
            assert!(axis > 0.9999, "{normal:?}");
        }
        assert!(!edges.indices.is_empty());
        let s = size(&body);
        assert!(s.y > 0.4 && s.y < 1.4, "{s:?}");
    }
    // The three variants have their own silhouettes.
    let sizes: Vec<Vector3> = (0..TOWER_VARIANTS)
        .map(|v| size(&tower_mesh(v).0))
        .collect();
    assert!(
        sizes[1].x > sizes[0].x && sizes[1].y < sizes[0].y,
        "civic block: wide, low"
    );
    assert!(
        sizes[2].x < sizes[0].x && sizes[2].y > sizes[0].y,
        "needle: slim, tall"
    );
}

#[test]
fn edge_strips_on_edges() {
    for v in 0..TOWER_VARIANTS {
        let (body, edges) = tower_mesh(v);
        // The boxes' edges: axis-aligned triangle edges (face diagonals
        // are not).
        let mut segments = Vec::new();
        for [a, b, c] in triangles(&body) {
            for (p, q) in [(a, b), (b, c), (c, a)] {
                let d = q - p;
                let aligned = [d.x, d.y, d.z].iter().filter(|x| x.abs() > 1e-6).count() == 1;
                if aligned {
                    segments.push((p, q));
                }
            }
        }
        let distance = |x: Vector3| {
            segments
                .iter()
                .map(|&(p, q)| {
                    let d = q - p;
                    let t = ((x - p).dot(d) / d.dot(d)).clamp(0.0, 1.0);
                    (x - (p + d * t)).length()
                })
                .fold(f32::MAX, f32::min)
        };
        for &x in &edges.vertices {
            assert!(
                distance(x) <= STRIP_WIDTH + STRIP_LIFT + 1e-4,
                "variant {v}: {x:?}"
            );
        }
        // Every strip faces away from the building's axis, off its face.
        for [a, b, c] in triangles(&edges) {
            let normal = (b - a).cross(c - a);
            let mid = (a + b + c) / 3.0;
            assert!(normal.dot(Vector3::new(mid.x, 0.0, mid.z)) > 0.0);
        }
    }
}

#[test]
fn closed_and_outward() {
    for v in 0..TOWER_VARIANTS {
        let (body, _) = tower_mesh(v);
        // Closed: every edge is shared by exactly two triangles, once in
        // each direction.
        let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
        for t in body.indices.chunks(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
            }
        }
        assert!(edges.values().all(|&n| n == 0), "variant {v}");
        // Outward: the signed volume is the boxes' volume (a flipped face
        // would take some away).
        let volume: f32 = triangles(&body)
            .map(|[a, b, c]| a.dot(b.cross(c)) / 6.0)
            .sum();
        assert!(volume > 0.01, "variant {v}: {volume}");
    }
}

#[test]
fn out_of_range_variant_is_zero() {
    assert_eq!(tower_mesh(TOWER_VARIANTS), tower_mesh(0));
    assert_eq!(tower_mesh(200), tower_mesh(0));
}

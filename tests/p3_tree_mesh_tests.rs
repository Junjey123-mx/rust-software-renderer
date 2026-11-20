//! Project 3 clover tree: closed solids wound outwards, within the
//! triangle budget, base at the origin with +Y up, and three variants of
//! the same kind.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::solar::attach::prop_meshes::{TREE_VARIANTS, tree_mesh, tree_parts};
use std::collections::HashMap;

/// Every edge used once in each direction (closed, consistently wound)
/// and a positive signed volume (wound outwards).
pub fn assert_closed_outward(m: &Mesh, what: &str) {
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    let mut volume = 0.0;
    for t in m.indices.chunks(3) {
        for k in 0..3 {
            *directed.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
        let (a, b, c) = (m.vertices[t[0]], m.vertices[t[1]], m.vertices[t[2]]);
        volume += a.dot(b.cross(c)) / 6.0;
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(count, 1, "{what}: edge {a}-{b} repeated");
        assert_eq!(directed.get(&(b, a)), Some(&1), "{what}: edge {a}-{b} open");
    }
    assert!(volume > 0.0, "{what}: inward ({volume})");
}

#[test]
fn closed_and_outward() {
    for v in 0..TREE_VARIANTS {
        let [trunk, canopy] = tree_parts(v);
        assert_closed_outward(&trunk, &format!("trunk {v}"));
        // The canopy is several cones: closed one by one.
        let cones = canopy.triangle_count() / 12;
        for c in 0..cones {
            let tris = &canopy.indices[c * 36..(c + 1) * 36];
            let cone = Mesh::new(canopy.vertices.clone(), tris.to_vec()).unwrap();
            assert_closed_outward(&cone, &format!("cone {c} of tree {v}"));
        }
    }
}

#[test]
fn triangle_budget() {
    for v in 0..TREE_VARIANTS {
        let n = tree_mesh(v).triangle_count();
        assert!((20..=120).contains(&n), "variant {v}: {n}");
    }
}

#[test]
fn base_at_origin_up_y() {
    for v in 0..TREE_VARIANTS {
        let m = tree_mesh(v);
        let min = m.vertices.iter().fold(Vector3::one() * f32::MAX, |a, p| {
            Vector3::new(a.x.min(p.x), a.y.min(p.y), a.z.min(p.z))
        });
        let max = m.vertices.iter().fold(Vector3::one() * f32::MIN, |a, p| {
            Vector3::new(a.x.max(p.x), a.y.max(p.y), a.z.max(p.z))
        });
        assert_eq!(min.y, 0.0, "variant {v} stands on the origin");
        assert!(
            (0.75..1.2).contains(&max.y),
            "about one unit tall: {}",
            max.y
        );
        let width = (max.x - min.x).max(max.z - min.z);
        assert!(max.y > 1.2 * width, "taller than wide");
        assert!(
            (min.x + max.x).abs() < 0.05 && (min.z + max.z).abs() < 0.05,
            "centered"
        );
    }
}

#[test]
fn variants_same_kind() {
    let shapes: Vec<Vec<Vector3>> = (0..TREE_VARIANTS).map(|v| tree_mesh(v).vertices).collect();
    for a in 0..shapes.len() {
        for b in a + 1..shapes.len() {
            assert_ne!(shapes[a], shapes[b], "variants {a} and {b} differ");
        }
    }
    // Every variant is a trunk under a canopy of cones.
    for v in 0..TREE_VARIANTS {
        let [trunk, canopy] = tree_parts(v);
        let trunk_top = trunk.vertices.iter().map(|p| p.y).fold(0.0, f32::max);
        let canopy_top = canopy.vertices.iter().map(|p| p.y).fold(0.0, f32::max);
        assert!(canopy_top > 2.0 * trunk_top);
        assert_eq!(canopy.triangle_count() % 12, 0);
    }
    // Out of range: variant 0.
    assert_eq!(tree_mesh(TREE_VARIANTS).vertices, tree_mesh(0).vertices);
    assert_eq!(tree_mesh(255).indices, tree_mesh(0).indices);
}

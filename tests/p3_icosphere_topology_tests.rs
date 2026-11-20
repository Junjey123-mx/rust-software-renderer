//! Project 3 icosphere: exact triangle counts per level, a closed surface
//! (every edge shared by two faces, Euler characteristic 2), unit
//! directions and the outward winding of `sphere.obj`.

use rust_software_renderer::solar::planet::icosphere::{
    IcoMesh, MAX_LEVEL, icosahedron, icosphere, subdivide, to_mesh,
};
use std::collections::HashMap;

fn edges(m: &IcoMesh) -> HashMap<(u32, u32), usize> {
    let mut count = HashMap::new();
    for t in &m.indices {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    count
}

fn closed(m: &IcoMesh) -> bool {
    edges(m).values().all(|&n| n == 2)
}

#[test]
fn triangle_counts_320_1280_5120_20480() {
    assert_eq!(icosahedron().indices.len(), 20);
    for (level, tris) in [(2, 320), (3, 1280), (4, 5120), (5, 20480)] {
        assert_eq!(icosphere(level).unwrap().indices.len(), tris, "{level}");
    }
}

#[test]
fn euler_characteristic_is_2() {
    for level in 0..=MAX_LEVEL {
        let m = icosphere(level).unwrap();
        let v = m.positions.len() as i64;
        let e = edges(&m).len() as i64;
        let f = m.indices.len() as i64;
        assert_eq!(v - e + f, 2, "level {level}");
    }
}

#[test]
fn every_edge_shared_by_two_faces() {
    for level in 0..=4 {
        assert!(closed(&icosphere(level).unwrap()), "level {level}");
    }
    // A mesh with an orphan edge (one face removed by hand) fails it.
    let mut open = subdivide(&icosahedron());
    open.indices.pop();
    assert!(!closed(&open));
}

#[test]
fn outward_winding() {
    let m = icosphere(3).unwrap();
    for t in &m.indices {
        let [a, b, c] = t.map(|i| m.positions[i as usize]);
        let normal = (b - a).cross(c - a);
        assert!(normal.dot(a + b + c) > 0.0, "{t:?}");
    }
    // The pipeline mesh keeps it: three corners per triangle, same order.
    let mesh = to_mesh(&m, |_| 2.0);
    assert_eq!(mesh.len(), 3 * m.indices.len());
    for tri in mesh.chunks(3) {
        let (a, b, c) = (tri[0].position, tri[1].position, tri[2].position);
        assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
        assert!((a.length() - 2.0).abs() < 1e-5);
    }
}

#[test]
fn unit_length_vertices() {
    for level in 0..=MAX_LEVEL {
        let m = icosphere(level).unwrap();
        for p in &m.positions {
            assert!((p.length() - 1.0).abs() < 1e-5, "level {level}: {p:?}");
        }
    }
    // Evenly spread: the shortest edge at level 4 is not far below the
    // longest.
    let m = icosphere(4).unwrap();
    let lengths: Vec<f32> = edges(&m)
        .keys()
        .map(|&(a, b)| (m.positions[a as usize] - m.positions[b as usize]).length())
        .collect();
    let (min, max) = lengths
        .iter()
        .fold((f32::MAX, 0.0f32), |(lo, hi), &l| (lo.min(l), hi.max(l)));
    assert!(min / max > 0.75, "{min} / {max}");
}

#[test]
fn level_6_rejected() {
    assert!(icosphere(6).is_err());
    assert!(icosphere(MAX_LEVEL).is_ok());
}

//! Project 3 boulder: the second surface model, told apart from the tree
//! by its shape and triangle count, closed and wound outwards, and varied
//! by its seed (0 included).

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::solar::attach::prop_meshes::{rock_mesh, tree_mesh};
use std::collections::HashMap;

fn assert_closed_outward(m: &Mesh, what: &str) {
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
    // Each face looks away from the solid's center (star-shaped boulder).
    let center = m.vertices.iter().fold(Vector3::zero(), |a, &p| a + p) / m.vertices.len() as f32;
    for t in m.indices.chunks(3) {
        let (a, b, c) = (m.vertices[t[0]], m.vertices[t[1]], m.vertices[t[2]]);
        let normal = (b - a).cross(c - a);
        assert!(
            normal.dot((a + b + c) / 3.0 - center) > 0.0,
            "{what}: face turned in"
        );
    }
}

fn bbox(m: &Mesh) -> (Vector3, Vector3) {
    m.vertices.iter().fold(
        (Vector3::one() * f32::MAX, Vector3::one() * f32::MIN),
        |(lo, hi), p| {
            (
                Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        },
    )
}

#[test]
fn distinct_from_tree() {
    let (rock, tree) = (rock_mesh(3), tree_mesh(1));
    assert_eq!(rock.triangle_count(), 80);
    assert_ne!(rock.triangle_count(), tree.triangle_count());
    // A rock is wider than tall; a tree is taller than wide.
    let ((rl, rh), (tl, th)) = (bbox(&rock), bbox(&tree));
    assert!(rh.y - rl.y < rh.x - rl.x, "flattened boulder");
    assert!(th.y - tl.y > th.x - tl.x, "upright tree");
}

#[test]
fn closed_and_outward() {
    for seed in [0, 1, 42, u32::MAX] {
        assert_closed_outward(&rock_mesh(seed), &format!("rock {seed}"));
    }
}

#[test]
fn seeded_variation() {
    assert_eq!(rock_mesh(9).vertices, rock_mesh(9).vertices);
    assert_ne!(rock_mesh(9).vertices, rock_mesh(10).vertices);
    // Seed 0 is a valid rock, partly sunk: its base just under the origin.
    let (lo, hi) = bbox(&rock_mesh(0));
    assert!(lo.y < 0.0 && lo.y > -0.2 && hi.y > 0.5, "{lo:?} {hi:?}");
    assert!(
        rock_mesh(0)
            .vertices
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    );
}

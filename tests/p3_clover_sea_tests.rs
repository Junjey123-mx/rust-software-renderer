//! Project 3 clover sea: one connected ocean, every peak above it, a coast
//! band around each mountain, and the same surface for the mesh and the
//! collisions.

use raylib::math::Vector3;
use rust_software_renderer::solar::physics::SurfaceSampler;
use rust_software_renderer::solar::planet::HeightField;
use rust_software_renderer::solar::planet::clover::CloverField;
use rust_software_renderer::solar::planet::icosphere::{IcoMesh, icosphere};
use std::collections::VecDeque;

/// Neighbors of every vertex of `mesh`.
fn neighbors(mesh: &IcoMesh) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); mesh.positions.len()];
    for t in &mesh.indices {
        for k in 0..3 {
            let (a, b) = (t[k] as usize, t[(k + 1) % 3] as usize);
            adj[a].push(b);
            adj[b].push(a);
        }
    }
    adj
}

/// Connected groups of the vertices where `keep` holds.
fn components(mesh: &IcoMesh, keep: impl Fn(Vector3) -> bool) -> usize {
    let adj = neighbors(mesh);
    let mut seen = vec![false; mesh.positions.len()];
    let mut groups = 0;
    for start in 0..mesh.positions.len() {
        if seen[start] || !keep(mesh.positions[start]) {
            continue;
        }
        groups += 1;
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(v) = queue.pop_front() {
            for &w in &adj[v] {
                if !seen[w] && keep(mesh.positions[w]) {
                    seen[w] = true;
                    queue.push_back(w);
                }
            }
        }
    }
    groups
}

#[test]
fn sea_is_connected() {
    let mesh = icosphere(3).unwrap();
    for seed in (0..24).chain([99, 2024]) {
        let f = CloverField::new(seed);
        assert_eq!(components(&mesh, |n| !f.is_land(n)), 1, "seed {seed}");
        // Land only near the clovers (lobes may surface as islets), never
        // from the seabed.
        for &n in &mesh.positions {
            if f.is_land(n) {
                let near = f.centers.iter().any(|&c| n.dot(c) > 0.85);
                assert!(near, "seed {seed}: land far from every clover at {n:?}");
            }
        }
    }
}

#[test]
fn peaks_above_sea() {
    for seed in [0, 7, 99] {
        let f = CloverField::new(seed);
        for (i, &c) in f.centers.iter().enumerate() {
            // The lobes' core lies around the center: some of it is land.
            let land = (0..64).any(|k| {
                let a = k as f32 / 64.0 * std::f32::consts::TAU;
                let t = c.cross(Vector3::new(a.cos(), 0.3, a.sin())).normalized();
                f.is_land((c + t * 0.12).normalized())
            });
            assert!(land, "seed {seed} clover {i}");
        }
    }
    // A sea at +1 R drowns every peak.
    let mut drowned = CloverField::new(7);
    drowned.sea = 1.0;
    let mesh = icosphere(3).unwrap();
    assert!(mesh.positions.iter().all(|&n| !drowned.is_land(n)));
}

#[test]
fn coast_band_exists() {
    let f = CloverField::new(7);
    let mesh = icosphere(5).unwrap();
    let (mut shore, mut beach) = (0, 0);
    for &n in &mesh.positions {
        let d = f.coast_distance(n);
        assert!(d.is_finite());
        assert_eq!(d > 0.0, f.is_land(n), "{n:?}");
        if (-0.02..0.0).contains(&d) {
            shore += 1;
        }
        if (0.0..0.02).contains(&d) {
            beach += 1;
        }
    }
    assert!(shore > 50 && beach > 50, "shore {shore}, beach {beach}");
}

#[test]
fn collision_radius_matches_surface() {
    let f = CloverField::new(11);
    let field = HeightField::Clover(f);
    let sampler = SurfaceSampler::Field { base: 1.45, field };
    for &n in &icosphere(3).unwrap().positions {
        let r = sampler.radius_at(n);
        assert_eq!(r, 1.45 * field.surface_radius(n));
        // Never under the sea, which the mesh draws at the same radius.
        assert!(r >= 1.45 * (1.0 + f.sea) - 1e-6);
        assert!(r <= sampler.max_radius() + 1e-6);
    }
}

//! Project 3 clover waterfalls: a ribbon anchored on a lobe's edge and at
//! the sea, of finite width, within its triangle budget and never floating
//! off the slope; no ribbon without sea to fall into.

use raylib::math::Vector3;
use rust_software_renderer::solar::attach::waterfalls::{
    WATERFALL_GAP, WATERFALL_WIDTH, Waterfall, waterfall_from,
};
use rust_software_renderer::solar::planet::CloverField;

const SEEDS: [u32; 3] = [7, 99, 2024];
const EPS: f32 = 1e-3;

fn all() -> Vec<(CloverField, Waterfall)> {
    SEEDS
        .iter()
        .flat_map(|&seed| {
            let field = CloverField::new(seed);
            (0..4).map(move |c| (field, waterfall_from(&field, c, seed).unwrap()))
        })
        .collect()
}

/// Height of `p` over the ground (or the sea) right below it.
fn over_surface(field: &CloverField, p: Vector3) -> f32 {
    let n = p.normalized();
    p.length() - 1.0 - field.height(n).max(field.sea)
}

#[test]
fn endpoints_touch_terrain_and_sea() {
    for (field, w) in all() {
        let (top, bottom) = (w.top.normalized(), w.bottom.normalized());
        assert!(field.height(top) > field.sea + 0.01, "top on the lobe");
        assert!((over_surface(&field, w.top) - WATERFALL_GAP).abs() < EPS);
        // The bottom is at the water, within one descent step of the coast.
        assert!(!field.is_land(bottom), "bottom in the sea");
        assert!(field.coast_distance(bottom) > -0.01);
        assert!((over_surface(&field, w.bottom) - WATERFALL_GAP).abs() < EPS);
        assert!((w.top - w.bottom).length() > 0.02, "it falls some way");
    }
}

#[test]
fn finite_widths() {
    for (_, w) in all() {
        assert_eq!(w.width, WATERFALL_WIDTH);
        assert!(
            w.mesh
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
        );
        for rung in w.mesh.vertices.chunks(2) {
            let width = (rung[1] - rung[0]).length();
            assert!(
                (width - WATERFALL_WIDTH).abs() < 0.2 * WATERFALL_WIDTH,
                "{width}"
            );
        }
    }
}

#[test]
fn triangle_budget() {
    for (_, w) in all() {
        let triangles = w.mesh.indices.len() / 3;
        assert!((24..=48).contains(&triangles), "{triangles}");
        assert!(w.mesh.indices.iter().all(|&i| i < w.mesh.vertices.len()));
        // Every face looks away from the planet.
        for t in w.mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| w.mesh.vertices[t[k]]);
            assert!((b - a).cross(c - a).dot(a) > 0.0);
        }
    }
}

#[test]
fn no_floating_segment() {
    for (field, w) in all() {
        let v = &w.mesh.vertices;
        // Every vertex, and the middle of every rung and of every segment,
        // lies `WATERFALL_GAP` over the surface.
        let mut points: Vec<Vector3> = v.clone();
        points.extend(v.chunks(2).map(|r| (r[0] + r[1]) * 0.5));
        points.extend(v.windows(3).step_by(2).map(|s| (s[0] + s[2]) * 0.5));
        for p in points {
            let h = over_surface(&field, p);
            assert!(h > 0.0 && h < WATERFALL_GAP + 0.004, "{h}");
        }
    }
}

#[test]
fn same_seed_same_waterfall() {
    let field = CloverField::new(7);
    assert_eq!(waterfall_from(&field, 2, 7), waterfall_from(&field, 2, 7));
}

#[test]
fn no_sea_no_waterfall() {
    // Sea below the seabed: no coast to fall to, so no ribbon at all.
    let mut field = CloverField::new(7);
    field.sea = -0.5;
    assert!(waterfall_from(&field, 0, 7).is_err());
    assert!(waterfall_from(&CloverField::new(7), 4, 7).is_err());
}

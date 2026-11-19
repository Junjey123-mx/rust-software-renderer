//! The royal crown of The King's Crown in the solar scene.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::solar::royal_crown::{CrownSpec, MAIN_POINTS, along, outward};
use rust_software_renderer::solar::shapes::{End, loft, octahedron};
use std::collections::HashMap;

/// Every directed edge has exactly one reverse twin: a closed surface whose
/// triangles all wind the same way.
fn is_closed(mesh: &Mesh) -> bool {
    let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
    for t in mesh.indices.chunks(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a, b)).or_default() += 1;
        }
    }
    edges
        .iter()
        .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
}

fn triangles(mesh: &Mesh) -> impl Iterator<Item = [Vector3; 3]> + '_ {
    mesh.indices.chunks(3).map(|t| {
        [
            mesh.vertices[t[0]],
            mesh.vertices[t[1]],
            mesh.vertices[t[2]],
        ]
    })
}

fn normal(t: &[Vector3; 3]) -> Vector3 {
    (t[1] - t[0]).cross(t[2] - t[0])
}

fn centroid(t: &[Vector3; 3]) -> Vector3 {
    (t[0] + t[1] + t[2]) / 3.0
}

/// Finite positions, no degenerate triangle.
fn is_sound(mesh: &Mesh) -> bool {
    mesh.vertices
        .iter()
        .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
        && triangles(mesh).all(|t| normal(&t).length() > 1e-7)
}

/// Every triangle of a convex piece faces away from its vertex mean.
fn faces_out_of(mesh: &Mesh) -> bool {
    let mean =
        mesh.vertices.iter().fold(Vector3::zero(), |a, &p| a + p) / mesh.vertices.len() as f32;
    triangles(mesh).all(|t| normal(&t).dot(centroid(&t) - mean) > 0.0)
}

#[test]
fn lofts_and_octahedra_are_closed_outward_solids() {
    let square = |y: f32, s: f32| {
        vec![
            Vector3::new(s, y, 0.0),
            Vector3::new(0.0, y, s),
            Vector3::new(-s, y, 0.0),
            Vector3::new(0.0, y, -s),
        ]
    };
    let blade = loft(
        &[square(0.0, 1.0), square(0.5, 1.2)],
        End::Cap,
        End::Apex(Vector3::new(0.0, 2.0, 0.0)),
        Vector3::new(0.0, 0.4, 0.0),
    );
    let gem = octahedron(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(0.3, 0.0, 0.0),
        Vector3::new(0.0, 0.5, 0.0),
        Vector3::new(0.0, 0.0, 0.2),
    );
    for mesh in [&blade, &gem] {
        assert!(is_closed(mesh) && is_sound(mesh) && faces_out_of(mesh));
    }
    assert_eq!(gem.triangle_count(), 8);
    assert_eq!(blade.triangle_count(), 8 + 2 + 4);
    // An open end leaves its ring's edges unmatched.
    let tube = loft(
        &[square(0.0, 1.0), square(1.0, 1.0)],
        End::Open,
        End::Cap,
        Vector3::new(0.0, 0.5, 0.0),
    );
    assert!(!is_closed(&tube) && faces_out_of(&tube));
}

#[test]
fn the_royal_crown_has_five_to_seven_main_points_of_several_heights() {
    let spec = CrownSpec::royal();
    let main: Vec<_> = spec.main_spikes().collect();
    assert_eq!(main.len(), MAIN_POINTS);
    assert!((5..=7).contains(&main.len()));
    let mut heights: Vec<i32> = main.iter().map(|s| (s.height * 1000.0) as i32).collect();
    heights.sort();
    heights.dedup();
    assert!(heights.len() >= 3, "{heights:?}");
    // The front point is the tallest and the crown is mirror-symmetric.
    let tallest = main
        .iter()
        .max_by(|a, b| a.height.total_cmp(&b.height))
        .unwrap();
    assert_eq!(tallest.azimuth, 0.0);
    for s in &spec.spikes {
        let mirror = spec
            .spikes
            .iter()
            .find(|m| (outward(m.azimuth) - outward(-s.azimuth)).length() < 1e-4)
            .expect("every point has a mirror twin");
        assert_eq!((mirror.height, mirror.width), (s.height, s.width));
    }
    assert_eq!(spec.gems.len(), MAIN_POINTS);
    assert!(
        spec.spikes.len() > main.len(),
        "small points between the main ones"
    );
}

#[test]
fn every_piece_is_a_sound_closed_mesh_facing_outwards() {
    let spec = CrownSpec::royal();
    let band = spec.band.mesh();
    assert!(is_closed(&band) && is_sound(&band));
    assert_eq!(band.triangle_count(), spec.band.segments * 8);
    // The band is a ring: each face looks away from the middle of its own
    // cross-section.
    let middle = spec.band.latitude.sin();
    for t in triangles(&band) {
        let c = centroid(&t);
        let azimuth = c.x.atan2(c.z);
        let section =
            outward(azimuth) * spec.band.radius_at(middle) + Vector3::new(0.0, middle, 0.0);
        assert!(normal(&t).dot(c - section) > 0.0, "band face at {c:?}");
    }
    for s in &spec.spikes {
        let blade = s.mesh(&spec.band);
        assert!(is_closed(&blade) && is_sound(&blade) && faces_out_of(&blade));
        if let Some(f) = s.finial_mesh(&spec.band) {
            assert!(is_closed(&f) && is_sound(&f) && faces_out_of(&f));
        }
    }
    for g in &spec.gems {
        let gem = g.mesh(&spec.band);
        assert!(is_closed(&gem) && is_sound(&gem) && faces_out_of(&gem));
    }
}

#[test]
fn the_band_sits_on_the_upper_star_and_the_points_rise_outwards() {
    let spec = CrownSpec::royal();
    let band = spec.band.mesh();
    let (bottom, top) = (spec.band.bottom(), spec.band.top());
    for p in &band.vertices {
        if (p.y - bottom).abs() < 1e-5 {
            assert!(
                p.length() < 0.97,
                "lower edge sunk into the sphere: {}",
                p.length()
            );
        } else {
            assert!(
                p.length() > 1.0,
                "upper edge outside the sphere: {}",
                p.length()
            );
        }
    }
    assert!(bottom > 0.5 && top < 0.95, "on the northern cap");
    for s in &spec.spikes {
        let (axis, tip) = (s.axis(), s.tip(&spec.band));
        assert!(
            axis.y > 0.9 && axis.dot(outward(s.azimuth)) > 0.0,
            "up and slightly out"
        );
        assert!(axis.dot(along(s.azimuth)).abs() < 1e-6, "no sideways tilt");
        assert!(tip.y > top && tip.length() > 1.1, "{tip:?}");
    }
}

#[test]
fn open_gaps_separate_the_points_so_the_star_shows_between_them() {
    let spec = CrownSpec::royal();
    let mut points = spec.spikes.clone();
    points.sort_by(|a, b| a.azimuth.total_cmp(&b.azimuth));
    let rim = spec.band.radius_at(spec.band.top());
    for (i, a) in points.iter().enumerate() {
        let b = &points[(i + 1) % points.len()];
        let gap = (b.azimuth - a.azimuth).rem_euclid(std::f32::consts::TAU) * rim;
        let clear =
            gap - (a.width + b.width) / 2.0 * rust_software_renderer::solar::royal_crown::BULGE;
        assert!(
            clear > 0.03,
            "points {i} and {} touch: {clear}",
            (i + 1) % points.len()
        );
    }
}

use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::royal_crown::RoyalCrown;

#[test]
fn the_crown_mesh_is_closed_per_material_and_reproducible() {
    let spec = CrownSpec::royal();
    for mesh in [spec.gold_mesh(), spec.jewel_mesh()] {
        assert!(is_closed(&mesh) && is_sound(&mesh));
    }
    let (a, b) = (RoyalCrown::royal(), RoyalCrown::royal());
    let positions = |v: &[rust_software_renderer::geometry::Vertex]| {
        v.iter().map(|c| c.position).collect::<Vec<_>>()
    };
    assert_eq!(positions(&a.gold), positions(&b.gold));
    assert_eq!(positions(&a.jewels), positions(&b.jewels));
    assert_eq!(a.gold.len() % 3, 0);
    assert_eq!(a.jewels.len() % 3, 0);
    // A clone shares the vertex lists instead of copying them.
    let c = a.clone();
    assert!(std::sync::Arc::ptr_eq(&a.gold, &c.gold));
}

#[test]
fn the_main_points_are_spread_evenly_around_the_rim() {
    let spec = CrownSpec::royal();
    let tips: Vec<Vector3> = spec.main_spikes().map(|s| s.tip(&spec.band)).collect();
    let step = std::f32::consts::TAU / MAIN_POINTS as f32;
    for (i, s) in spec.main_spikes().enumerate() {
        assert!((s.azimuth - step * i as f32).abs() < 1e-5);
        let t = tips[i];
        let seen = t.x.atan2(t.z).rem_euclid(std::f32::consts::TAU);
        assert!((seen - s.azimuth.rem_euclid(std::f32::consts::TAU)).abs() < 1e-4);
    }
    for (i, a) in tips.iter().enumerate() {
        let b = tips[(i + 1) % tips.len()];
        assert!((*a - b).length() > 0.3, "neighbouring tips apart");
    }
}

#[test]
fn the_built_crown_is_scaled_to_the_course_sphere_and_stays_light() {
    let crown = RoyalCrown::royal();
    let gold = crown.spec.gold_mesh();
    let expected: Vec<Vector3> = gold
        .indices
        .iter()
        .map(|&i| gold.vertices[i] * SPHERE_RADIUS)
        .collect();
    let built: Vec<Vector3> = crown.gold.iter().map(|v| v.position).collect();
    assert_eq!(built, expected);
    let reach = crown.extent * SPHERE_RADIUS;
    let farthest = crown
        .gold
        .iter()
        .chain(crown.jewels.iter())
        .map(|v| v.position.length())
        .fold(0.0, f32::max);
    assert!((farthest - reach).abs() < 1e-5, "{farthest} vs {reach}");
    let tallest = crown
        .spec
        .main_spikes()
        .map(|s| s.tip(&crown.spec.band).length())
        .fold(0.0, f32::max);
    assert!(
        crown.extent > tallest && crown.extent < 1.8,
        "{}",
        crown.extent
    );
    let n = crown.triangle_count();
    assert!(n > 300 && n < 800, "{n} triangles");
}

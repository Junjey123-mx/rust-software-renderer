//! Project 3 Hearts terrain: terraces in the mesh itself (not only in the
//! color), finite normals on the steps, plateaus for the city and the same
//! field for the collisions.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::planet::field::NORMAL_EPS;
use rust_software_renderer::solar::planet::geometry::relief_mesh;
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn field() -> HeartsField {
    HeartsField::new(7)
}

/// `count` directions spread over the sphere (Fibonacci lattice).
fn directions(count: usize) -> Vec<Vector3> {
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    (0..count)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
            let r = (1.0 - y * y).sqrt();
            let a = golden * i as f32;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

#[test]
fn terraces_visible_in_vertex_heights() {
    let f = field();
    let mesh = relief_mesh(4, 1.0, &HeightField::Hearts(f)).unwrap();
    let levels: Vec<f32> = (0..=f.steps as usize)
        .map(|k| k as f32 / f.steps * f.height)
        .collect();
    let mut used = vec![0usize; levels.len()];
    let mut on_level = 0;
    for v in &mesh {
        let h = v.position.length() - 1.0;
        if let Some(k) = levels.iter().position(|l| (h - l).abs() < 1e-4) {
            used[k] += 1;
            on_level += 1;
        }
    }
    // Most vertices sit exactly on a terrace, on at least three levels.
    assert!(on_level * 2 > mesh.len(), "{on_level} of {}", mesh.len());
    assert!(used.iter().filter(|&&c| c > 50).count() >= 3, "{used:?}");
}

#[test]
fn finite_normals_at_steps() {
    let f = field();
    let h = HeightField::Hearts(f);
    let mut steepest: f32 = 0.0;
    for n in directions(4000) {
        let normal = h.normal(n, NORMAL_EPS);
        assert!(normal.x.is_finite() && normal.y.is_finite() && normal.z.is_finite());
        assert!((normal.length() - 1.0).abs() < 1e-3);
        assert!(normal.dot(n) > 0.2, "outward even on a ramp");
        steepest = steepest.max(f.slope(n));
    }
    // There are steps (steep ramps), but no vertical wall.
    assert!(steepest > 0.2 && steepest < 5.0, "{steepest}");
}

#[test]
fn plateau_mask_nonempty() {
    let f = field();
    let dirs = directions(4000);
    let flat = dirs.iter().filter(|&&n| f.is_plateau(n)).count();
    assert!(flat * 10 > dirs.len() * 4, "{flat} of {}", dirs.len());
    assert!(flat < dirs.len(), "the canyon walls are not plateau");
    // Canyons: the lowest terrace is there too.
    assert!(dirs.iter().any(|&n| f.height(n) < 0.3 * f.height));
    assert!(dirs.iter().any(|&n| f.height(n) > 0.7 * f.height));
}

#[test]
fn collision_shares_field() {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap();
    let flat = s.collision_shapes()[1].radius;
    s.set_relief(1, HeightField::Hearts(field())).unwrap();
    let shaped = s.collision_shapes()[1].radius;
    let r = s.radius_of(1);
    assert!((shaped - flat - r * field().max_height()).abs() < 1e-4);
    // The mesh, the collisions and local flight read the same field.
    let relief = s.relief_of(1).unwrap();
    assert_eq!(relief.field, HeightField::Hearts(field()));
    let h = HeightField::Hearts(field());
    for v in relief.mesh(2).iter().take(200) {
        let n = v.position.normalized();
        let expected = rust_software_renderer::solar::SPHERE_RADIUS * h.surface_radius(n);
        assert!((v.position.length() - expected).abs() < 1e-4);
    }
}

#[test]
fn zero_steps_rejected() {
    let f = HeartsField {
        steps: 0.0,
        ..field()
    };
    assert!(f.validate().is_err());
    assert!(relief_mesh(2, 1.0, &HeightField::Hearts(f)).is_err());
    assert!(field().validate().is_ok());
}

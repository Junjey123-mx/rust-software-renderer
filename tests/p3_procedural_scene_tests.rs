//! Project 3 procedural geometry, one rubric each (C76), on the planets as
//! the scene builds them: the icosphere's subdivision (20·4^n triangles,
//! whatever the relief), the deformation by the height field (whatever the
//! subdivision; `Flat` is no deformation), the seeded distribution of the
//! surface models and their pose riding the planet's spin.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::planet::geometry::{LOD_LEVELS, relief_mesh};
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};
use std::collections::HashSet;

const CLOVER: PlanetId = PlanetId("clover");
const HEARTS: PlanetId = PlanetId("hearts");

fn build(spec: &SystemSpec) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
    .unwrap()
}

/// True when some vertex leaves the base sphere of radius `r` by more than
/// 0.1 % of it: a deformation, not a resampled sphere.
fn deformed(mesh: &[Vertex], r: f32) -> bool {
    mesh.iter()
        .any(|v| (v.position.length() - r).abs() > 1e-3 * r)
}

fn bits(p: Vector3) -> [u32; 3] {
    [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
}

#[test]
fn subdivision_independent_of_relief() {
    let s = build(&SystemSpec::project3());
    let fields = [
        HeightField::Flat,
        s.relief_of(s.body_index(CLOVER).unwrap()).unwrap().field,
        s.relief_of(s.body_index(HEARTS).unwrap()).unwrap().field,
    ];
    for level in LOD_LEVELS {
        let triangles = 20 * 4usize.pow(level as u32);
        for field in &fields {
            let mesh = relief_mesh(level, SPHERE_RADIUS, field).unwrap();
            assert_eq!(mesh.len(), 3 * triangles, "level {level} {field:?}");
        }
        // The scene's own levels: the same counts on both relief planets.
        for id in [CLOVER, HEARTS] {
            let relief = s.relief_of(s.body_index(id).unwrap()).unwrap();
            assert_eq!(relief.mesh(level).len(), 3 * triangles, "{id:?} {level}");
        }
    }
}

#[test]
fn relief_independent_of_subdivision() {
    let s = build(&SystemSpec::project3());
    for id in [CLOVER, HEARTS] {
        let relief = s.relief_of(s.body_index(id).unwrap()).unwrap();
        for &level in &LOD_LEVELS {
            let mesh = relief.mesh(level);
            assert!(deformed(mesh, SPHERE_RADIUS), "{id:?} {level}");
            // Every vertex at the field's height in its direction.
            for v in mesh.iter() {
                let n = v.position.normalized();
                let ground = SPHERE_RADIUS * relief.field.surface_radius(n);
                assert!((v.position.length() - ground).abs() < 1e-5);
            }
        }
        // The coarse level's vertices are in the finest one, bit for bit.
        let fine: HashSet<[u32; 3]> = relief.mesh(5).iter().map(|v| bits(v.position)).collect();
        assert!(
            relief
                .mesh(2)
                .iter()
                .all(|v| fine.contains(&bits(v.position)))
        );
    }
    // Negative case: the flat field resamples the sphere and does not count.
    for level in LOD_LEVELS {
        let flat = relief_mesh(level, SPHERE_RADIUS, &HeightField::Flat).unwrap();
        assert!(!deformed(&flat, SPHERE_RADIUS));
    }
    let hearts = HeightField::Hearts(HeartsField::new(11));
    assert!(deformed(
        &relief_mesh(2, SPHERE_RADIUS, &hearts).unwrap(),
        SPHERE_RADIUS
    ));
}

#[test]
fn seed_distribution() {
    let sites = |s: &SolarScene| {
        (
            s.props.as_ref().unwrap().sites.clone(),
            s.city.as_ref().unwrap().sites.clone(),
        )
    };
    let spec = SystemSpec::project3();
    let (a, b) = (build(&spec), build(&spec));
    assert_eq!(sites(&a), sites(&b), "same seed, same places");
    let (trees, city) = sites(&a);
    assert!(
        trees.len() > 200 && city.len() > 400,
        "{} {}",
        trees.len(),
        city.len()
    );
    // Another seed on each planet moves most of its models.
    let mut other = spec.clone();
    for p in &mut other.planets {
        if p.id == CLOVER || p.id == HEARTS {
            p.seed += 1;
        }
    }
    let (trees2, city2) = sites(&build(&other));
    let moved = |a: &[Vector3], b: &[Vector3]| {
        a.iter()
            .filter(|p| !b.iter().any(|q| (**p - *q).length() < 1e-3))
            .count() as f32
            / a.len() as f32
    };
    assert!(moved(&trees, &trees2) > 0.9, "{}", moved(&trees, &trees2));
    assert!(moved(&city, &city2) > 0.9, "{}", moved(&city, &city2));
}

#[test]
fn orientation_on_rotation() {
    let mut s = build(&SystemSpec::project3());
    let i = s.body_index(HEARTS).unwrap();
    let landmark = s.landmark.as_ref().unwrap().clone();
    let site = s.city.as_ref().unwrap().sites[0];
    let mut poses = Vec::new();
    for t in [0.0, 7.5, 31.0] {
        s.clock.time = t;
        s.update();
        let host = s.bodies[i].transform;
        let up = host.to_world_dir(landmark.up);
        let radial = host.to_world_dir(landmark.dir);
        // The pyramid stands on its plateau: "up" is the ground's normal,
        // the same angle off the radial at every time.
        assert!((up.dot(radial) - landmark.up.dot(landmark.dir)).abs() < 1e-5);
        assert!(landmark.up.dot(landmark.dir) > 0.95);
        // A building of the city keeps its place on the planet.
        let building = host.to_world_dir(site);
        assert!((host.to_object_dir(building) - site).length() < 1e-5);
        poses.push((up, building, host.position));
    }
    // ... while the planet turns and moves, carrying both.
    for w in poses.windows(2) {
        assert!(w[0].0.dot(w[1].0) < 0.9999, "the pyramid turned");
        assert!(w[0].1.dot(w[1].1) < 0.9999, "the building turned");
        assert_ne!(w[0].2, w[1].2, "the planet moved");
    }
}

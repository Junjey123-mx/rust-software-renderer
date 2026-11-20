//! Project 3 scale profile: the spacecraft is the unit; planets are 3-6
//! spacecraft radii and the star 3-5 radii of the largest planet, checked
//! on world radii converted from the real meshes.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::solar::scene::SHIP_SCALE;
use rust_software_renderer::solar::ship::{MESH_CENTER, MESH_RADIUS};
use rust_software_renderer::solar::system::{ScaleProfile, SpecError, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes};

/// Largest distance of a vertex from `center`.
fn mesh_radius(mesh: &[Vertex], center: Vector3) -> f32 {
    mesh.iter()
        .map(|v| (v.position - center).length())
        .fold(0.0, f32::max)
}

#[test]
fn ship_radius_from_mesh() {
    let meshes = SolarMeshes::load().unwrap();
    // Measured from `MESH_CENTER`, the point the ship flies around.
    let measured = mesh_radius(&meshes.ship, MESH_CENTER);
    assert!(
        (measured - MESH_RADIUS).abs() < 0.01,
        "ship mesh radius {measured}"
    );
    let profile = ScaleProfile::project3();
    assert_eq!(profile.ship_radius, MESH_RADIUS * SHIP_SCALE);
    assert!((profile.ship_radius - 0.319).abs() < 1e-3);
}

#[test]
fn sphere_obj_conversion() {
    let meshes = SolarMeshes::load().unwrap();
    let local = mesh_radius(&meshes.sphere, Vector3::zero());
    assert!(
        (local - SPHERE_RADIUS).abs() < 1e-3,
        "sphere.obj radius {local}"
    );
    // World radius = transform scale x mesh radius: 1.3 gives 1.126.
    let scale = ScaleProfile::world_scale_for(1.3 * SPHERE_RADIUS, SPHERE_RADIUS);
    assert!((scale - 1.3).abs() < 1e-6);
    let ship = ScaleProfile::world_scale_for(0.319, MESH_RADIUS);
    assert!((ship - SHIP_SCALE).abs() < 1e-3);
}

#[test]
fn preproject3_fails_ratio_check_as_documented() {
    // Crimson Entrance (0.52) is 1.6 spacecraft radii, below the 3 the
    // profile asks for: the pre-project system is not to scale.
    let err = ScaleProfile::project3()
        .check(&SystemSpec::preproject3())
        .unwrap_err();
    match err {
        SpecError::OutOfRange("planet radius / ship radius", ratio) => {
            assert!((ratio - 0.6 * SPHERE_RADIUS / (MESH_RADIUS * SHIP_SCALE)).abs() < 1e-4)
        }
        other => panic!("{other:?}"),
    }
}

/// A system that satisfies the profile: planets 3.4-5.5 spacecraft radii,
/// the star 3.1 times the largest, orbits at 1 u/s.
fn proportioned() -> SystemSpec {
    let mut spec = SystemSpec::preproject3();
    let profile = ScaleProfile::project3();
    spec.planets[0].scale = 3.4 * profile.ship_radius / SPHERE_RADIUS;
    spec.planets[1].scale = 5.5 * profile.ship_radius / SPHERE_RADIUS;
    spec.star.scale = 3.1 * spec.planets[1].scale;
    for p in &mut spec.planets {
        p.orbit_speed = 1.0;
    }
    spec
}

#[test]
fn ratios_with_tolerance() {
    let profile = ScaleProfile::project3();
    profile.check(&proportioned()).unwrap();
    // Exactly at the limits (and within the tolerance) passes.
    let mut spec = proportioned();
    spec.planets[0].scale = 3.0 * profile.ship_radius / SPHERE_RADIUS * (1.0 - 5e-4);
    spec.star.scale = 5.0 * spec.planets[1].scale;
    profile.check(&spec).unwrap();
    // A star smaller than three times the largest planet does not.
    let mut spec = proportioned();
    spec.star.scale = 2.9 * spec.planets[1].scale;
    assert!(matches!(
        profile.check(&spec),
        Err(SpecError::OutOfRange(
            "star radius / largest planet radius",
            _
        ))
    ));
    // Nor a planet over six spacecraft, nor an orbit faster than 1 u/s.
    let mut spec = proportioned();
    spec.planets[1].scale = 6.1 * profile.ship_radius / SPHERE_RADIUS;
    assert!(profile.check(&spec).is_err());
    let mut spec = proportioned();
    spec.planets[0].orbit_speed = -1.2;
    assert_eq!(
        profile.check(&spec),
        Err(SpecError::OutOfRange("planet.orbit_speed", -1.2))
    );
}

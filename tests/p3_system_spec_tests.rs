//! Project 3 system specifications: `SystemSpec::preproject3` reproduces the
//! hard-wired pre-project system and `validate` rejects bad data with its
//! own error, never a panic.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::planet::PlanetMaterial;
use rust_software_renderer::solar::system::spec::{FIFTH_PLANET, FifthPlanetStatus};
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

fn hardwired() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

#[test]
fn preproject3_matches_hardwired_values() {
    let spec = SystemSpec::preproject3();
    spec.validate().unwrap();
    let scene = hardwired();
    assert_eq!(scene.bodies.len(), 1 + spec.planets.len());

    let star = &scene.bodies[0];
    assert_eq!(spec.star.id, PlanetId(star.body().name()));
    assert_eq!(spec.star.scale, star.transform.scale);
    assert_eq!(spec.star.spin, star.spin_speed);
    assert_eq!(spec.star.material, PlanetMaterial::Lab6(star.material));
    assert!(star.orbit.is_none());

    for (p, b) in spec.planets.iter().zip(&scene.bodies[1..]) {
        let orbit = b.orbit.unwrap();
        assert_eq!(p.id, PlanetId(b.body().name()));
        assert_eq!(p.scale, b.transform.scale);
        assert!((p.radius() - b.transform.scale * SPHERE_RADIUS).abs() < 1e-6);
        assert_eq!(p.spin, b.spin_speed);
        assert_eq!(p.orbit_radius, orbit.radius);
        assert!((p.angular_speed() - orbit.speed).abs() < 1e-6, "{}", p.name);
        assert_eq!(p.phase, orbit.phase);
        assert_eq!(p.material, PlanetMaterial::Lab6(b.material));
        assert_eq!((p.mu, p.host), (0.0, None));
    }
    // Linear speeds of the pre-project orbits.
    assert!((spec.planets[0].orbit_speed - 1.5).abs() < 1e-6);
    assert!((spec.planets[1].orbit_speed - 1.364).abs() < 1e-6);
}

#[test]
fn duplicate_id_rejected() {
    let mut spec = SystemSpec::preproject3();
    spec.planets[1].id = spec.planets[0].id;
    assert_eq!(
        spec.validate(),
        Err(SpecError::DuplicateId(spec.planets[0].id))
    );
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].id = spec.star.id;
    assert_eq!(spec.validate(), Err(SpecError::DuplicateId(spec.star.id)));
}

#[test]
fn nan_radius_rejected() {
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].scale = f32::NAN;
    assert_eq!(spec.validate(), Err(SpecError::NonFinite("planet.scale")));
    let mut spec = SystemSpec::preproject3();
    spec.planets[1].orbit_radius = -2.0;
    assert_eq!(
        spec.validate(),
        Err(SpecError::OutOfRange("planet.orbit_radius", -2.0))
    );
    let mut spec = SystemSpec::preproject3();
    spec.star.scale = 0.0;
    assert_eq!(
        spec.validate(),
        Err(SpecError::OutOfRange("star.scale", 0.0))
    );
}

#[test]
fn negative_mu_rejected() {
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].mu = -0.1;
    assert_eq!(
        spec.validate(),
        Err(SpecError::OutOfRange("planet.mu", -0.1))
    );
    let mut spec = SystemSpec::preproject3();
    spec.star.mu = f32::INFINITY;
    assert_eq!(spec.validate(), Err(SpecError::NonFinite("star.mu")));
}

#[test]
fn unknown_host_rejected() {
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].host = Some(PlanetId("moon"));
    assert_eq!(
        spec.validate(),
        Err(SpecError::UnknownHost(PlanetId("moon")))
    );
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].host = Some(spec.planets[0].id);
    assert!(matches!(spec.validate(), Err(SpecError::UnknownHost(_))));
    let mut spec = SystemSpec::preproject3();
    spec.planets[0].host = Some(spec.planets[1].id);
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn fifth_is_the_approved_joker() {
    // C70 documented two concepts with none approved (four planets); the
    // author approved the Joker on 2026-11-20 and it is the fifth planet.
    assert_eq!(FIFTH_PLANET, FifthPlanetStatus::Approved("Joker"));
    let spec = SystemSpec::project3();
    assert_eq!(spec.planets.len(), 5, "the four suits and the Joker");
    let ids: Vec<&str> = spec.planets.iter().map(|p| p.id.0).collect();
    assert_eq!(ids, ["entrance", "clover", "hearts", "house", "joker"]);
    assert!(
        !ids.iter()
            .any(|id| id.contains("crystal") || id.contains("fifth"))
    );
    assert_eq!(spec.validate(), Ok(()));
}

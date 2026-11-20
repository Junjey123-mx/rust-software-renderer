//! Project 3 strangler step: the solar scene built from
//! `SystemSpec::preproject3()` is the pre-project scene, instance by
//! instance and pixel by pixel.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{SCENARIOS, scenario_scene};
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
use rust_software_renderer::solar::{BodyInstance, Orbit, SolarMeshes, SolarScene};

fn view() -> OrbitView {
    OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap()
}

/// The pre-project bodies wired by hand, as `default_system` did before
/// the spec existed (`ea3b4a8`).
fn hardwired_bodies() -> Vec<BodyInstance> {
    let mut crown = BodyInstance::new(Body::KingsCrown, Transform::new(Vector3::zero(), 1.3), 0.05);
    crown.material.set("loops", 0.0).unwrap();
    vec![
        crown,
        BodyInstance::new(
            Body::CrimsonEntrance,
            Transform::new(Vector3::zero(), 0.6),
            0.6,
        )
        .with_orbit(Orbit::new(3.0, 0.5, 0.0).unwrap()),
        BodyInstance::new(
            Body::HouseOfCards,
            Transform::new(Vector3::zero(), 0.9),
            0.35,
        )
        .with_orbit(Orbit::new(6.2, 0.22, std::f32::consts::PI).unwrap()),
    ]
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn from_spec_equals_default_system_a_j() {
    let light = Vector3::new(0.0, 1.0, 0.0);
    let spec = SolarScene::from_spec(
        SolarMeshes::load().unwrap(),
        view(),
        light,
        &SystemSpec::preproject3(),
    )
    .unwrap();
    let mut wired = SolarScene::new(SolarMeshes::load().unwrap(), view(), light);
    wired.bodies = hardwired_bodies();
    wired.update();
    assert_eq!(spec.bodies, wired.bodies);

    // Every scenario (built through `default_system`, now the spec) renders
    // the same pixels as the hand-wired bodies at the same instant.
    let config = SolarBenchConfig {
        width: 200,
        height: 150,
        ..SolarBenchConfig::default()
    };
    for (id, _) in SCENARIOS {
        let from_spec = scenario_scene(id, &config).unwrap();
        let mut by_hand = from_spec.clone();
        let mut bodies = hardwired_bodies();
        bodies.retain(|b| from_spec.bodies.iter().any(|s| s.body() == b.body()));
        by_hand.bodies = bodies;
        by_hand.update();
        assert_eq!(by_hand.bodies, from_spec.bodies, "{id}");
        let hash = |s: &SolarScene| {
            let mut fb = Framebuffer::new(config.width, config.height);
            s.render_frame(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions::default(),
            )
            .unwrap();
            fnv1a64(&fb.to_rgba_bytes())
        };
        assert_eq!(hash(&by_hand), hash(&from_spec), "{id}");
    }
}

#[test]
fn invalid_spec_is_an_error_not_a_partial_scene() {
    let light = Vector3::new(0.0, 1.0, 0.0);
    let mut spec = SystemSpec::preproject3();
    spec.planets[1].orbit_radius = f32::NAN;
    let scene = SolarScene::from_spec(SolarMeshes::load().unwrap(), view(), light, &spec);
    assert_eq!(
        scene.err(),
        Some(SpecError::NonFinite("planet.orbit_radius"))
    );
    // A valid moon is not supported yet: an error too.
    let mut spec = SystemSpec::preproject3();
    spec.planets[1].host = Some(PlanetId("entrance"));
    let scene = SolarScene::from_spec(SolarMeshes::load().unwrap(), view(), light, &spec);
    assert_eq!(scene.err(), Some(SpecError::Unsupported("planet.host")));
}

//! Project 3 live parameters: `--set <id>.<param>=<value>` changes body
//! values of the system spec (radius, orbit, spin, seed, gravity) before
//! the scene is built, and every other parameter still reaches the shader.

use raylib::math::Vector3;
use rust_software_renderer::app::parse_args;
use rust_software_renderer::celestial::cli::{extract, validate};
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::system::{SPEC_PARAMS, SpecError, SystemSpec};
use rust_software_renderer::solar::viewer::build_scene;
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

fn built(sets: &[(&str, f32)]) -> Result<SolarScene, String> {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let sets: Vec<(String, f32)> = sets.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    SolarScene::from_spec_with_sets(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        SystemSpec::preproject3(),
        &sets,
    )
}

#[test]
fn radius_override_applies() {
    let s = built(&[("entrance.radius", 0.8), ("crown.radius", 1.5)]).unwrap();
    assert!((s.radius_of(1) - 0.8).abs() < 1e-6);
    assert!((s.radius_of(0) - 1.5).abs() < 1e-6);
    let s = built(&[("house.orbit_radius", 7.0), ("house.orbit_speed", 2.1)]).unwrap();
    let orbit = s.bodies[2].orbit.unwrap();
    assert_eq!(orbit.radius, 7.0);
    assert!((orbit.speed - 0.3).abs() < 1e-6);
}

#[test]
fn seed_override_changes_spec() {
    let mut spec = SystemSpec::preproject3();
    assert_eq!(spec.apply_set("house.planet_seed", 7.0), Ok(true));
    assert_eq!(spec.planets[1].seed, 7);
    assert_eq!(spec.apply_set("entrance.mu", 0.25), Ok(true));
    assert_eq!(spec.planets[0].mu, 0.25);
    assert_eq!(
        spec.apply_set("house.planet_seed", 2.5),
        Err(SpecError::OutOfRange("planet_seed", 2.5))
    );
}

#[test]
fn nan_rejected() {
    let err = built(&[("entrance.radius", f32::NAN)]).err().unwrap();
    assert!(
        err.contains("entrance.radius") && err.contains("finite"),
        "{err}"
    );
}

#[test]
fn negative_rejected() {
    let err = built(&[("house.orbit_radius", -1.0)]).err().unwrap();
    assert!(
        err.contains("house.orbit_radius") && err.contains("out of range"),
        "{err}"
    );
    let err = built(&[("entrance.mu", -0.1)]).err().unwrap();
    assert!(err.contains("out of range"), "{err}");
    let err = built(&[("crown.orbit_radius", 3.0)]).err().unwrap();
    assert!(err.contains("not supported"), "{err}");
}

#[test]
fn unknown_id_rejected() {
    let err = built(&[("nope.radius", 1.0)]).err().unwrap();
    assert!(
        err.contains("nope.radius") && err.contains("no body"),
        "{err}"
    );
    let err = built(&[("entrance.unknown", 1.0)]).err().unwrap();
    assert!(err.contains("unknown"), "{err}");
}

#[test]
fn shader_params_still_route_to_apply_overrides() {
    // `seed` and `bump` belong to the laboratory 6 shaders.
    let s = built(&[("entrance.bump", 0.0), ("entrance.seed", 9.0)]).unwrap();
    assert_eq!(s.bodies[1].material.get("bump"), Some(0.0));
    assert_eq!(s.bodies[1].material.get("seed"), Some(9.0));
    // No spec parameter shadows a shader parameter of any body.
    for b in &s.bodies {
        for spec in b.material.specs() {
            assert!(!SPEC_PARAMS.contains(&spec.name), "{}", spec.name);
        }
    }
}

#[test]
fn the_command_line_changes_the_system_without_recompiling() {
    let line = "--scene solar --set entrance.radius=0.75 --set house.spin=1.0";
    let (lab6, rest) = extract(line.split_whitespace().map(str::to_string).collect()).unwrap();
    let cli = parse_args(rest).unwrap();
    validate(&cli, &lab6).unwrap();
    let s = build_scene(&cli, &lab6).unwrap();
    assert!((s.radius_of(1) - 0.75).abs() < 1e-6);
    assert_eq!(s.bodies[2].spin_speed, 1.0);
    assert!((s.bodies[1].transform.scale - 0.75 / SPHERE_RADIUS).abs() < 1e-6);
}

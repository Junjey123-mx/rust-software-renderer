//! Project 3 system at the project scale: planets 3-6 spacecraft radii,
//! the star 3-5 times the largest planet, the spacecraft as it was, and a
//! scene whose orbits keep every shell apart.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::scene::SHIP_SCALE;
use rust_software_renderer::solar::ship::MESH_RADIUS;
use rust_software_renderer::solar::system::{ScaleProfile, SpecError, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn build(spec: &SystemSpec) -> Result<SolarScene, SpecError> {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
}

#[test]
fn ratios_within_targets() {
    let spec = SystemSpec::project3();
    let profile = ScaleProfile::project3();
    profile.check(&spec).unwrap();
    for p in &spec.planets {
        let ratio = p.radius() / profile.ship_radius;
        assert!((3.0..=6.0).contains(&ratio), "{}: {ratio}", p.name);
        assert!(p.orbit_speed.abs() <= profile.max_orbit_speed);
    }
    let radii: Vec<f32> = spec.planets.iter().map(|p| p.radius()).collect();
    // C49 adds the clover planet (1.45) between the other two.
    let expected = [1.10, 1.45, 1.75];
    assert!(
        radii
            .iter()
            .zip(expected)
            .all(|(r, e)| (r - e).abs() < 1e-5)
    );
    // The scene builds, the orbits are the spec's and the shells apart.
    let s = build(&spec).unwrap();
    let layout = s.layout.clone().unwrap();
    assert_eq!(layout.radii, vec![14.0, 21.0, 40.0]);
    let mut edge = s.shell_of(0);
    for (i, r) in layout.radii.iter().enumerate() {
        assert!(r - s.shell_of(i + 1) >= edge + profile.min_gap);
        edge = r + s.shell_of(i + 1);
    }
    s.check_spawn().unwrap();
    assert_eq!(s.scale, Some(profile));
}

#[test]
fn star_3_to_5_times_largest_planet() {
    let spec = SystemSpec::project3();
    let star = spec.star.scale * rust_software_renderer::solar::SPHERE_RADIUS;
    let largest = spec.planets.iter().map(|p| p.radius()).fold(0.0, f32::max);
    let ratio = star / largest;
    assert!((3.0..=5.0).contains(&ratio), "{ratio}");
    assert!((star - 5.40).abs() < 1e-5);
    // A star at 2.9 times the largest planet is refused.
    let mut small = spec.clone();
    small.star.scale = 2.9 * largest / rust_software_renderer::solar::SPHERE_RADIUS;
    let err = build(&small).err().unwrap();
    assert!(
        matches!(err, SpecError::OutOfRange("star radius / largest planet radius", r) if (r - 2.9).abs() < 1e-3),
        "{err:?}"
    );
}

#[test]
fn ship_radius_unchanged() {
    let s = build(&SystemSpec::project3()).unwrap();
    assert_eq!(s.ship.radius(), MESH_RADIUS * SHIP_SCALE);
    assert_eq!(s.ship.radius(), ScaleProfile::project3().ship_radius);
    // It starts beyond the fifth slot, nose towards the star.
    let layout = s.layout.as_ref().unwrap();
    assert!(s.ship.position.z > layout.fifth_slot);
    assert!(s.ship.forward.z < 0.0);
}

#[test]
fn window_scene_is_the_scaled_system() {
    use rust_software_renderer::app::parse_args;
    use rust_software_renderer::celestial::cli::{extract, validate};
    use rust_software_renderer::solar::viewer::build_project3_scene;
    let args = |line: &str| {
        let (lab6, rest) = extract(line.split_whitespace().map(str::to_string).collect()).unwrap();
        let cli = parse_args(rest).unwrap();
        validate(&cli, &lab6).unwrap();
        (cli, lab6)
    };
    let (cli, lab6) = args("--scene solar --time 2");
    let s = build_project3_scene(&cli, &lab6).unwrap();
    let layout = s.layout.as_ref().unwrap();
    assert_eq!(layout.radii, vec![14.0, 21.0, 40.0]);
    assert!(s.extent() <= layout.world_radius);
    assert!((s.time() - 2.0).abs() < 1e-6);
    // `--set` values go through the layout: an orbit into the star's shell
    // is refused with the planet's name.
    let (cli, lab6) = args("--scene solar --set entrance.orbit_radius=10");
    let err = build_project3_scene(&cli, &lab6).unwrap_err().to_string();
    assert!(err.contains("entrance") && err.contains("needs"), "{err}");
}

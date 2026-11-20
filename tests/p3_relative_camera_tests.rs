//! Project 3 relative distances: local flight and quick travel scale with
//! the planet, the pre-project constants stay for the legacy scene, and
//! the ship starts (with its follow camera) clear of every shell.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::{
    CLEARANCE, FlightScale, LOCAL_LAP_SECONDS, LOCAL_RANGE, LOCAL_SPEED, RADIAL_SPEED,
};
use rust_software_renderer::solar::scene::{SHIP_SPAWN, spawn_beyond};
use rust_software_renderer::solar::system::{ScaleProfile, SpecError, SystemSpec};
use rust_software_renderer::solar::travel::WARP_MARGIN;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn build(spec: &SystemSpec) -> Result<SolarScene, SpecError> {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
}

#[test]
fn legacy_constants_preserved() {
    let legacy = FlightScale::legacy();
    assert_eq!(
        legacy,
        FlightScale {
            clearance: CLEARANCE,
            local_range: LOCAL_RANGE,
            local_speed: LOCAL_SPEED,
            radial_speed: RADIAL_SPEED,
            warp_margin: WARP_MARGIN,
        }
    );
    assert_eq!((CLEARANCE, LOCAL_RANGE, WARP_MARGIN), (0.5, 3.0, 1.2));
    assert_eq!((LOCAL_SPEED, RADIAL_SPEED), (0.5, 0.4));
    assert_eq!(SHIP_SPAWN, spawn_beyond(8.0));
    // The pre-project scene keeps them for every body.
    let s = build(&SystemSpec::preproject3()).unwrap();
    assert!(s.scale.is_none());
    for i in 0..s.bodies.len() {
        assert_eq!(s.flight_scale(i), legacy);
    }
}

#[test]
fn relative_scale_follows_the_planet() {
    let mut s = build(&SystemSpec::preproject3()).unwrap();
    let profile = ScaleProfile::project3();
    s.scale = Some(profile);
    for i in 1..s.bodies.len() {
        let r = s.radius_of(i);
        let f = s.flight_scale(i);
        assert_eq!(f, FlightScale::for_planet(&profile, r));
        assert!((f.clearance - (profile.ship_radius + 0.15 * r)).abs() < 1e-6);
        assert!(f.clearance < f.warp_margin && f.warp_margin < f.local_range);
        // A lap at the clearance height at full thrust.
        let lap = std::f32::consts::TAU * (r + f.clearance) / f.local_speed;
        assert!((lap - LOCAL_LAP_SECONDS).abs() < 1e-3);
    }
    // For a design planet (1.10): about 0.48 clearance, entry within 2.75.
    let f = FlightScale::for_planet(&profile, 1.10);
    assert!((f.clearance - 0.484).abs() < 1e-3 && (f.local_range - 2.75).abs() < 1e-6);
}

#[test]
fn spawn_outside_star_shell() {
    let s = build(&SystemSpec::preproject3()).unwrap();
    assert!(s.check_spawn().is_ok());
    let star = s.shell_of(0);
    assert!((s.ship.position - s.bodies[0].transform.position).length() > star);
    // The crown (1.7615 R) sets the star's shell, the ring House's.
    assert!((star - s.radius_of(0) * 1.7615).abs() < 1e-3);
    assert!((s.shell_of(2) - s.radius_of(2) * 2.134).abs() < 1e-3);
    assert_eq!(s.shell_of(1), s.radius_of(1));
}

#[test]
fn follow_camera_outside_bodies_at_spawn() {
    let s = build(&SystemSpec::preproject3()).unwrap();
    let eye = s.follow.camera(&s.ship).eye;
    for i in 0..s.bodies.len() {
        let d = (eye - s.bodies[i].transform.position).length();
        assert!(d > s.shell_of(i), "eye inside body {i}");
    }
}

#[test]
fn spawn_inside_a_shell_is_an_error() {
    // House of Cards on an orbit through the start point.
    let mut spec = SystemSpec::preproject3();
    spec.planets[1].orbit_radius = 9.0;
    assert!(matches!(
        build(&spec).err(),
        Some(SpecError::OutOfRange("ship spawn room", _))
    ));
    // A star that swallows it.
    let mut spec = SystemSpec::preproject3();
    spec.star.scale = 6.0;
    assert!(build(&spec).is_err());
}

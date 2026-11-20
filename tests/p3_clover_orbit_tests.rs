//! Project 3 clover planet in the solar scene (C49): its orbit and spin,
//! its slot between Crimson Entrance and House of Cards, a close flight
//! that never enters the ground, quick travel with F3/F4 and the level of
//! detail that follows the distance.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::flight::FlightMode;
use rust_software_renderer::solar::input::{Look, PRESSED, SolarAction};
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const CLOVER: PlanetId = PlanetId("clover");

fn view() -> OrbitView {
    OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap()
}

fn build(spec: &SystemSpec) -> Result<SolarScene, SpecError> {
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view(),
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
}

fn scene() -> SolarScene {
    build(&SystemSpec::project3()).unwrap()
}

fn clover(s: &SolarScene) -> usize {
    s.body_index(CLOVER)
        .expect("the clover planet is in the system")
}

/// Distance from the ship's center to the clover's ground right below it.
fn above_ground(s: &SolarScene) -> f32 {
    let i = clover(s);
    let t = s.bodies[i].transform;
    let d = s.ship.position - t.position;
    let ground = s.radius_of(i)
        * s.relief_of(i)
            .unwrap()
            .field
            .surface_radius(t.to_object_dir(d).normalized());
    d.length() - ground
}

#[test]
fn clover_orbits_and_spins() {
    let mut s = scene();
    let i = clover(&s);
    assert_eq!(s.bodies[i].name, "Clover Haven");
    assert!(s.relief_of(i).is_some(), "drawn with its relief");
    let r = s.layout.as_ref().unwrap().radii[i - 1];
    let mut previous: Option<Vector3> = None;
    for t in [0.0f32, 5.0, 40.0] {
        s.clock.time = t;
        s.update();
        let p = s.bodies[i].transform.position - s.center;
        assert!((p.length() - r).abs() < 1e-3, "t {t}: {}", p.length());
        assert!(p.y.abs() < 1e-5);
        assert!((s.bodies[i].transform.yaw - 0.12 * t).abs() < 1e-5);
        if let Some(q) = previous {
            assert!((p - q).length() > 0.5, "it moves along its orbit");
        }
        previous = Some(p);
    }
}

#[test]
fn orbit_between_crimson_and_house() {
    let s = scene();
    let ids: Vec<&str> = s.bodies.iter().map(|b| b.id.0).collect();
    assert_eq!(ids, ["crown", "entrance", "clover", "house"]);
    let layout = s.layout.clone().unwrap();
    assert_eq!(layout.radii, vec![14.0, 21.0, 40.0]);
    // Shells (body, relief, props and rings) apart, with the profile gap.
    let gap = s.scale.unwrap().min_gap;
    for i in 1..layout.radii.len() {
        let inner = layout.radii[i - 1] + s.shell_of(i);
        let outer = layout.radii[i] - s.shell_of(i + 1);
        assert!(outer - inner >= gap - 1e-4, "{} -> {}", ids[i], ids[i + 1]);
    }
    // The clover's shell covers its peaks and props.
    let i = clover(&s);
    let max = s.relief_of(i).unwrap().field.max_height();
    assert!(s.shell_of(i) > s.radius_of(i) * (1.0 + max));
    s.check_spawn().unwrap();
}

#[test]
fn clover_orbit_invading_house_is_refused() {
    let mut spec = SystemSpec::project3();
    let clover = spec.planets.iter_mut().find(|p| p.id == CLOVER).unwrap();
    clover.orbit_radius = 38.0;
    let err = build(&spec).err().unwrap();
    assert!(
        matches!(err, SpecError::OrbitOverlap { planet: PlanetId("house"), radius, .. } if radius == 40.0),
        "{err:?}"
    );
}

#[test]
fn close_flight_no_crash() {
    let config = SolarBenchConfig {
        width: 160,
        height: 120,
        threads: 2,
        ..SolarBenchConfig::default()
    };
    // Local flight over the clover's land (scenario K): forward, down and
    // sideways for 8 s, drawing every half second.
    let mut s = scenario_scene_on("K", &config, true).unwrap();
    assert!(matches!(s.flight, FlightMode::Local { .. }));
    let mut fb = Framebuffer::new(config.width, config.height);
    let options = ShadedOptions {
        threads: 2,
        ..ShadedOptions::default()
    };
    let actions = [
        [SolarAction::Forward, SolarAction::Descend],
        [SolarAction::Forward, SolarAction::StrafeRight],
    ];
    for frame in 0..480 {
        step(
            &mut s,
            &actions[frame / 120 % 2],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        assert!(s.ship.position.x.is_finite());
        assert!(above_ground(&s) > s.ship.radius(), "frame {frame}");
        if frame % 30 == 0 {
            s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
                .unwrap();
        }
    }
    // Space flight straight at it: stopped at its highest ground.
    let mut s = scene();
    s.gravity = false;
    let i = clover(&s);
    let c = s.bodies[i].transform.position;
    s.ship.position = c + Vector3::new(0.0, 0.0, s.radius_of(i) + 6.0);
    s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    let peak = s.radius_of(i) * (1.0 + s.relief_of(i).unwrap().field.max_height());
    for _ in 0..240 {
        step(
            &mut s,
            &[SolarAction::Forward],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        let d = (s.ship.position - s.bodies[i].transform.position).length();
        assert!(d - s.ship.radius() > peak - 1e-3, "{d}");
    }
}

#[test]
fn warp_f4_arrives_safely() {
    // F1-F4 follow the spec order: star, Crimson Entrance, clover, House.
    let key = |k| PRESSED.iter().find(|&&(key, _)| key == k).unwrap().1;
    assert_eq!(key(KeyboardKey::KEY_F3), SolarAction::Warp(2));
    assert_eq!(key(KeyboardKey::KEY_F4), SolarAction::Warp(3));
    for (index, name) in [(2, "Clover Haven"), (3, "House of Cards")] {
        let mut s = scene();
        s.clock.time = 6.0;
        s.update();
        let messages = apply_toggles(&mut s, &[SolarAction::Warp(index)]);
        assert_eq!(messages, [format!("Warped to {name}")]);
        let to_body = (s.bodies[index].transform.position - s.ship.position).normalized();
        assert!(to_body.dot(s.ship.forward) > 0.999, "nose towards {name}");
        // Clear of every body's collision shape, and still clear a second
        // later.
        for _ in 0..60 {
            for (b, shape) in s.collision_shapes().iter().enumerate() {
                let d = (s.ship.position - s.bodies[b].transform.position).length();
                assert!(d > shape.radius, "{name}: body {b} at {d}");
            }
            step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
        }
    }
}

#[test]
fn lod_changes_near_far() {
    let config = SolarBenchConfig {
        width: 400,
        height: 300,
        threads: 2,
        ..SolarBenchConfig::default()
    };
    let options = ShadedOptions {
        threads: 2,
        ..ShadedOptions::default()
    };
    let level = |id| {
        let s = scenario_scene_on(id, &config, true).unwrap();
        let mut fb = Framebuffer::new(config.width, config.height);
        s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
            .unwrap();
        let i = clover(&s);
        s.last_stats()
            .per_body
            .iter()
            .find(|b| b.index == i)
            .unwrap()
            .lod
    };
    let (near, far) = (level("K").unwrap(), level("K2").unwrap());
    assert!(near > far, "near L{near}, far L{far}");
    assert_eq!(far, 2);
}

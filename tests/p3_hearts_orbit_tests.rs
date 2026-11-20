//! Project 3 Hearts planet in the solar scene (C61): its slot between the
//! clover and House of Cards, its pyramid and city on board, a close
//! flight that never enters the ground, quick travel with F3 and the frame
//! time the benchmark measures for it.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{run_solar_benchmark_on, scenario_scene_on};
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::AttachmentKind;
use rust_software_renderer::solar::input::{Look, PRESSED, SolarAction};
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

const HEARTS: PlanetId = PlanetId("hearts");

fn build(spec: &SystemSpec) -> Result<SolarScene, SpecError> {
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
}

fn hearts(s: &SolarScene) -> usize {
    s.body_index(HEARTS)
        .expect("the hearts planet is in the system")
}

/// Distance from the ship's center to the hearts ground right below it.
fn above_ground(s: &SolarScene) -> f32 {
    let i = hearts(s);
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
fn hearts_between_clover_and_house() {
    let s = build(&SystemSpec::project3()).unwrap();
    let ids: Vec<&str> = s.bodies.iter().map(|b| b.id.0).collect();
    assert_eq!(ids, ["crown", "entrance", "clover", "hearts", "house"]);
    let layout = s.layout.clone().unwrap();
    assert_eq!(layout.radii, vec![14.0, 21.0, 29.5, 40.0]);
    let gap = s.scale.unwrap().min_gap;
    for i in 1..layout.radii.len() {
        let inner = layout.radii[i - 1] + s.shell_of(i);
        let outer = layout.radii[i] - s.shell_of(i + 1);
        assert!(outer - inner >= gap - 1e-4, "{} -> {}", ids[i], ids[i + 1]);
    }
    // Drawn with its terraces, pyramid and city, all inside its shell.
    let i = hearts(&s);
    assert_eq!(s.bodies[i].name, "Neon Hearts");
    assert!(s.relief_of(i).is_some());
    let on_hearts = |kind| {
        s.attachments
            .iter()
            .any(|a| a.kind == kind && a.host == HEARTS)
    };
    assert!(on_hearts(AttachmentKind::Landmark) && on_hearts(AttachmentKind::City));
    let landmark = s.landmark.as_ref().unwrap();
    assert!(s.shell_of(i) >= s.radius_of(i) * landmark.reach / SPHERE_RADIUS);
    assert!(!s.city.as_ref().unwrap().layers.is_empty());
    s.check_spawn().unwrap();
}

#[test]
fn hearts_after_house_in_the_spec_is_data() {
    // The order of the spec is data: hearts moved after House of Cards on
    // an orbit past House's shell is valid and keeps its place in the body
    // list (the scaled system takes explicit orbits only).
    let mut spec = SystemSpec::project3();
    let at = spec.planets.iter().position(|p| p.id == HEARTS).unwrap();
    let mut moved = spec.planets.remove(at);
    moved.orbit_radius = 50.0;
    spec.planets.push(moved.clone());
    let s = build(&spec).unwrap();
    let ids: Vec<&str> = s.bodies.iter().map(|b| b.id.0).collect();
    assert_eq!(ids, ["crown", "entrance", "clover", "house", "hearts"]);
    let radii = s.layout.clone().unwrap().radii;
    assert!(radii[3] > radii[2], "{radii:?}");
    // Its old orbit, now inside House of Cards', is refused.
    moved.orbit_radius = 29.5;
    *spec.planets.last_mut().unwrap() = moved;
    let err = build(&spec).err().unwrap();
    assert!(
        matches!(err, SpecError::OrbitOverlap { planet: HEARTS, .. }),
        "{err:?}"
    );
}

#[test]
fn close_flight_and_warp() {
    let config = SolarBenchConfig {
        width: 160,
        height: 120,
        threads: 2,
        ..SolarBenchConfig::default()
    };
    // From the M pose, straight down at the city for 4 s: stopped above
    // its ground and its pyramid.
    let mut s = scenario_scene_on("M", &config, true).unwrap();
    s.gravity = true;
    for frame in 0..240 {
        step(
            &mut s,
            &[SolarAction::Forward],
            &[],
            Look::default(),
            1.0 / 60.0,
        );
        assert!(s.ship.position.x.is_finite());
        assert!(above_ground(&s) > s.ship.radius() - 1e-3, "frame {frame}");
    }
    // F3 travels to it, the fourth body of the spec after the star.
    let key = |k| PRESSED.iter().find(|&&(key, _)| key == k).unwrap().1;
    assert_eq!(key(KeyboardKey::KEY_F3), SolarAction::Warp(3));
    let mut s = build(&SystemSpec::project3()).unwrap();
    s.clock.time = 6.0;
    s.update();
    let i = hearts(&s);
    assert_eq!(i, 3);
    let messages = apply_toggles(&mut s, &[SolarAction::Warp(i)]);
    assert_eq!(messages, ["Warped to Neon Hearts".to_string()]);
    let to_body = (s.bodies[i].transform.position - s.ship.position).normalized();
    assert!(to_body.dot(s.ship.forward) > 0.999);
    for _ in 0..60 {
        for (b, shape) in s.collision_shapes().iter().enumerate() {
            let d = (s.ship.position - s.bodies[b].transform.position).length();
            assert!(d > shape.radius, "body {b} at {d}");
        }
        step(&mut s, &[], &[], Look::default(), 1.0 / 60.0);
    }
}

#[test]
fn fps_measured_in_benchmark() {
    // 320x240: big enough for the close planet to pass level 2.
    let config = SolarBenchConfig {
        width: 320,
        height: 240,
        threads: 2,
        warmup: 0,
        rounds: 1,
        ..SolarBenchConfig::default()
    };
    let rows = run_solar_benchmark_on(&config, true).unwrap();
    for id in ["M", "M2"] {
        let row = rows.iter().find(|r| r.id == id).unwrap();
        assert!(row.median_ms.is_finite() && row.median_ms > 0.0, "{id}");
        assert!(row.stats.triangles_submitted > 0, "{id}");
    }
    // Up close the planet takes a finer level than from afar.
    let level = |id| {
        let row = rows.iter().find(|r| r.id == id).unwrap();
        let entry = row.lod_levels.split(';').find(|e| e.starts_with("hearts:"));
        entry
            .unwrap()
            .trim_start_matches("hearts:L")
            .parse::<u8>()
            .unwrap()
    };
    assert!(level("M") > level("M2"));
}

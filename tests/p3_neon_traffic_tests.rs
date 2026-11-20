//! Project 3 neon traffic over Hearts (C65): as many ships as the spec
//! says, flying nose first along their routes, carried by the planet's
//! orbit and spin, and never through the player's ship or camera.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::AttachmentKind;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::system::spec::AttachmentSpec;
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
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

fn with_ships(ships: u8) -> SystemSpec {
    let mut spec = SystemSpec::project3();
    let hearts = spec.planets.iter_mut().find(|p| p.id == HEARTS).unwrap();
    hearts.attachments = vec![AttachmentSpec::Traffic {
        ships,
        cycles: 0,
        seed: 11,
    }];
    spec
}

#[test]
fn ship_count_from_spec() {
    let s = build(&SystemSpec::project3()).unwrap();
    assert_eq!(s.traffic.as_ref().unwrap().ships.len(), 6);
    let host = s
        .attachments
        .iter()
        .find(|a| a.kind == AttachmentKind::Traffic)
        .unwrap()
        .host;
    assert_eq!(host, HEARTS);
    let s = build(&with_ships(3)).unwrap();
    assert_eq!(s.traffic.as_ref().unwrap().ships.len(), 3);
    // Negative case: no ships, no traffic and nothing to draw.
    let s = build(&with_ships(0)).unwrap();
    assert!(s.traffic.is_none());
    assert!(
        !s.attachments
            .iter()
            .any(|a| a.kind == AttachmentKind::Traffic)
    );
    assert!(
        !s.effect_draws(800, 600, 1)
            .iter()
            .any(|d| matches!(d.shader, EffectShader::Neon(_)))
    );
    // More than a planet takes is refused.
    let err = build(&with_ships(200)).err().unwrap();
    assert!(
        matches!(err, SpecError::OutOfRange("planet.traffic.ships", _)),
        "{err:?}"
    );
}

#[test]
fn oriented_along_tangent() {
    let config = SolarBenchConfig {
        width: 800,
        height: 600,
        threads: 1,
        ..SolarBenchConfig::default()
    };
    let s = scenario_scene_on("M", &config, true).unwrap();
    let traffic = s.traffic.as_ref().unwrap();
    let host = s.bodies[s.body_index(HEARTS).unwrap()].transform;
    let time = s.clock.time;
    for (i, (_, ship)) in traffic.ships.iter().enumerate() {
        let (center, forward, up) = traffic.world_pose(i, time, &host);
        // The nose (-Z of the mesh) along the heading, the top up.
        let m = ship.matrix(center, forward, up);
        let back = Vector3::new(m.m8, m.m9, m.m10).normalized();
        let top = Vector3::new(m.m4, m.m5, m.m6).normalized();
        assert!(back.dot(-forward) > 0.9999 && top.dot(up) > 0.9999, "{i}");
        // ... which is where it is a moment later (on the host's frame).
        let (next, _, _) = traffic.world_pose(i, time + 0.01, &host);
        assert!((next - center).normalized().dot(forward) > 0.99, "{i}");
    }
    // The ships in view are drawn with the neon shader.
    let neon = s
        .effect_draws(config.width, config.height, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Neon(_)))
        .count();
    assert!(neon > 0 && neon <= traffic.ships.len(), "{neon}");
}

#[test]
fn moves_with_host() {
    let mut s = build(&SystemSpec::project3()).unwrap();
    let i = s.body_index(HEARTS).unwrap();
    let mut last = None;
    for t in [0.0f32, 5.0, 30.0] {
        s.clock.time = t;
        s.update();
        let host = s.bodies[i].transform;
        let traffic = s.traffic.as_ref().unwrap();
        for (k, (path, _)) in traffic.ships.iter().enumerate() {
            // In the planet's own frame the ship is on its route.
            let (center, _, _) = traffic.world_pose(k, t, &host);
            let local = host.to_object_dir(center - host.position) / host.scale;
            let expected = path.pose(t, &traffic.field, SPHERE_RADIUS).pos;
            assert!((local - expected).length() < 1e-3, "ship {k} at {t}");
            // Close to the planet whatever its orbit.
            let altitude = (center - host.position).length() / s.radius_of(i) - 1.0;
            assert!(altitude > 0.29 && altitude < 0.7, "{altitude}");
        }
        assert_ne!(last, Some(host.position), "the planet moves");
        last = Some(host.position);
    }
}

#[test]
fn never_inside_player_camera_near_plane() {
    let config = SolarBenchConfig {
        width: 160,
        height: 120,
        threads: 1,
        ..SolarBenchConfig::default()
    };
    // The player hovering over the pyramid (scenario M), held there in the
    // planet's frame while the traffic flies a minute.
    let mut s = scenario_scene_on("M", &config, true).unwrap();
    let i = s.body_index(HEARTS).unwrap();
    let host = s.bodies[i].transform;
    let local = |v: Vector3| host.to_object_dir(v);
    let (p, f, u) = (
        local(s.ship.position - host.position),
        local(s.ship.forward),
        local(s.ship.up),
    );
    for step in 0..240 {
        let t = 6.0 + step as f32 * 0.25;
        s.clock.time = t;
        s.update();
        let host = s.bodies[i].transform;
        s.ship.position = host.position + host.to_world_dir(p);
        s.ship.forward = host.to_world_dir(f);
        s.ship.up = host.to_world_dir(u);
        let eye = s.follow.camera(&s.ship);
        let traffic = s.traffic.as_ref().unwrap();
        for (k, (_, ship)) in traffic.ships.iter().enumerate() {
            let (center, _, _) = traffic.world_pose(k, t, &host);
            let to_eye = (center - eye.eye).length();
            assert!(
                to_eye > ship.radius() + eye.near,
                "ship {k} at {t}: {to_eye}"
            );
            let to_player = (center - s.ship.position).length();
            assert!(
                to_player > ship.radius() + s.ship.radius(),
                "ship {k} at {t}: {to_player}"
            );
        }
    }
}

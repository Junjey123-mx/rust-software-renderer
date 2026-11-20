//! Project 3 lightcycles on Hearts (C67): loops over the ground that keep
//! clear of every building and of the pyramid, frozen by the pause and the
//! same for the same time.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::path::Path;
use rust_software_renderer::solar::attach::traffic::{
    CYCLE_ALTITUDE, CYCLE_MARGIN, Traffic, route_clear_of_buildings,
};
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::scene::TRAFFIC_CLEARANCE;
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

fn scene() -> SolarScene {
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
        &SystemSpec::project3(),
    )
    .unwrap()
}

fn matrices(s: &SolarScene) -> Vec<[f32; 16]> {
    let host = s.bodies[s.body_index(PlanetId("hearts")).unwrap()].transform;
    let traffic = s.traffic.as_ref().unwrap();
    (0..traffic.cycles.len())
        .map(|i| {
            let m = traffic.cycle_matrix(i, s.clock.time, &host);
            [
                m.m0, m.m1, m.m2, m.m3, m.m4, m.m5, m.m6, m.m7, m.m8, m.m9, m.m10, m.m11, m.m12,
                m.m13, m.m14, m.m15,
            ]
        })
        .collect()
}

#[test]
fn paused_frozen() {
    let mut s = scene();
    s.clock.time = 7.0;
    s.update();
    s.clock.paused = true;
    let before = matrices(&s);
    assert_eq!(before.len(), 4, "the spec's four cycles");
    for _ in 0..90 {
        s.clock.advance(1.0 / 60.0);
        s.update();
    }
    assert_eq!(matrices(&s), before);
    // Running again, they move.
    s.clock.paused = false;
    s.clock.advance(1.0 / 60.0);
    s.update();
    assert_ne!(matrices(&s), before);
}

#[test]
fn fixed_t_reproducible() {
    let (mut a, mut b) = (scene(), scene());
    for s in [&mut a, &mut b] {
        s.clock.time = 12.5;
        s.update();
    }
    assert_eq!(
        a.traffic.as_ref().unwrap().cycles,
        b.traffic.as_ref().unwrap().cycles
    );
    assert_eq!(matrices(&a), matrices(&b));
}

#[test]
fn stays_on_surface() {
    let s = scene();
    let traffic = s.traffic.as_ref().unwrap();
    for path in &traffic.cycles {
        assert!(matches!(path, Path::Ring { .. }));
        for k in 0..400 {
            let t = path.period() * k as f32 / 400.0;
            let p = path.pose(t, &traffic.field, SPHERE_RADIUS);
            let ground = SPHERE_RADIUS * traffic.field.surface_radius(p.up);
            let over = (p.pos.length() - ground) / SPHERE_RADIUS;
            assert!((over - CYCLE_ALTITUDE).abs() < 1e-4, "{over}");
            assert!(p.tangent.dot(p.up).abs() < 1e-4);
        }
    }
}

#[test]
fn clear_of_buildings() {
    let s = scene();
    let traffic = s.traffic.as_ref().unwrap();
    let sites = &s.city.as_ref().unwrap().sites;
    assert!(sites.len() > 400, "the whole city");
    let pyramid = s.landmark.as_ref().unwrap().dir;
    for path in &traffic.cycles {
        assert!(route_clear_of_buildings(path, sites, CYCLE_MARGIN));
        for k in 0..256 {
            let d = path.direction(path.period() * k as f32 / 256.0);
            assert!(d.dot(pyramid).acos() > TRAFFIC_CLEARANCE);
        }
    }
    // The check itself: a loop around a building is refused.
    let around = Path::Ring {
        axis: sites[0],
        radius: 0.02,
        speed: 1.0,
        phase: 0.0,
        altitude: CYCLE_ALTITUDE,
    };
    assert!(!route_clear_of_buildings(&around, sites, CYCLE_MARGIN));
}

#[test]
fn exhausted_tries_leave_the_cycle_out() {
    // A "city" on every lattice point: no loop fits, every cycle is left
    // out with a warning, and nothing panics.
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    let dense: Vec<Vector3> = (0..6000)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 6000.0;
            let r = (1.0 - y * y).sqrt();
            let a = golden * i as f32;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect();
    let meshes = SolarMeshes::load().unwrap();
    let field = HeightField::Hearts(HeartsField::new(11));
    let traffic =
        Traffic::new(&meshes, field, 0, 11, 0.1, 1.3, None).with_cycles(4, 11, &dense, None);
    assert!(traffic.cycles.is_empty());
    assert!(traffic.ships.is_empty());
}

//! The royal crown, the diamond ring, the exhaust and the piloting together
//! in the default solar system.

use raylib::math::{Matrix, Vector3};
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::card_ring::card_mesh;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::flight::{FlightMode, MAX_PITCH, MIN_PITCH, nose_elevation};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::{WORLD_RADIUS, step};
use rust_software_renderer::solar::{CameraMode, SolarMeshes, SolarScene};

const W: u32 = 400;
const H: u32 = 300;

fn system() -> SolarScene {
    let camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    let view = OrbitView::for_world(camera, WORLD_RADIUS, Viewport::new(W, H, 0.1)).unwrap();
    let mut s = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.camera_mode = CameraMode::Orbit;
    s.update();
    s
}

fn count(s: &SolarScene) -> (usize, usize, usize) {
    let draws = s.effect_draws(W, H, 1);
    let of = |f: fn(&EffectShader) -> bool| draws.iter().filter(|d| f(&d.shader)).count();
    (
        of(|e| matches!(e, EffectShader::Crown(_))),
        of(|e| matches!(e, EffectShader::Card(_))),
        of(|e| matches!(e, EffectShader::Flame(_))),
    )
}

fn apply(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

fn index_of(s: &SolarScene, body: Body) -> usize {
    s.bodies.iter().position(|b| b.body() == body).unwrap()
}

#[test]
fn the_default_system_wears_the_crown_deals_the_ring_and_fires_the_engines() {
    let s = system();
    let ring = s.cards.as_ref().unwrap().slots.len();
    assert_eq!(count(&s), (2, ring, 3));
    // The loops of Gate 10 are still there, untouched.
    assert!(!s.plasma.arcs.is_empty());
}

#[test]
fn effective_radii_keep_every_orbit_clear_for_a_whole_turn() {
    let mut s = system();
    let star = index_of(&s, Body::KingsCrown);
    let entrance = index_of(&s, Body::CrimsonEntrance);
    let house = index_of(&s, Body::HouseOfCards);
    let crowned = s.radius_of(star) * s.crown.as_ref().unwrap().extent;
    let mesh = card_mesh();
    let mut closest = f32::INFINITY;
    for k in 0..240 {
        s.clock.time = k as f32 * 2.5;
        s.update();
        let entrance_at = s.bodies[entrance].transform.position;
        let entrance_r = s.radius_of(entrance);
        let gap = (entrance_at - s.bodies[star].transform.position).length() - entrance_r;
        assert!(
            gap > crowned + 0.3,
            "crown {crowned} reaches Crimson Entrance ({gap})"
        );
        let draws = s.effect_draws(W, H, 1);
        for d in draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Card(_)))
        {
            for &p in &mesh.vertices {
                let q = apply(&d.model.model_matrix, p);
                closest = closest.min((q - entrance_at).length() - entrance_r);
                assert!((q - s.bodies[house].transform.position).length() > s.radius_of(house));
            }
        }
    }
    assert!(
        closest > 0.8,
        "a card comes within {closest} of Crimson Entrance"
    );
    let ring = s.radius_of(house) * s.cards.as_ref().unwrap().extent();
    let spawn = rust_software_renderer::solar::scene::SHIP_SPAWN;
    assert!(
        spawn.length() > 6.2 + ring + s.ship.radius(),
        "the ship spawns in the ring"
    );
    assert!(
        6.2 + ring <= WORLD_RADIUS,
        "the orbit camera frames the ring"
    );
}

#[test]
fn a_long_piloted_flight_stays_level_finite_and_carries_its_exhaust() {
    let mut s = system();
    s.camera_mode = CameraMode::Perspective;
    s.rig = CameraRig::Follow;
    let mut time = s.clock.time;
    for i in 0..600 {
        let t = i as f32 * 0.05;
        let mut actions = vec![SolarAction::Forward];
        if (t * 0.7).sin() > 0.3 {
            actions.push(SolarAction::StrafeRight);
        }
        if (t * 0.4).cos() > 0.5 {
            actions.push(SolarAction::Boost);
        }
        let look = Look {
            yaw: 0.02 * (t * 1.3).sin(),
            pitch: 0.03 * (t * 0.9).cos(),
        };
        step(&mut s, &actions, &[], look, 1.0 / 60.0);
        assert!(
            (s.clock.time - time - 1.0 / 60.0).abs() < 1e-4,
            "boost bends the clock"
        );
        time = s.clock.time;
        let ship = &s.ship;
        assert!(ship.position.x.is_finite() && ship.forward.dot(ship.up).abs() < 1e-4);
        let e = nose_elevation(ship, Vector3::new(0.0, 1.0, 0.0));
        assert!((MIN_PITCH - 1e-3..=MAX_PITCH + 1e-3).contains(&e));
        assert!((0.0..=1.0).contains(&ship.boost) && (0.0..=1.0).contains(&ship.thrust));
        for d in s.effect_draws(W, H, 1) {
            if matches!(d.shader, EffectShader::Flame(_)) {
                let base = apply(&d.model.model_matrix, Vector3::zero());
                assert!(
                    (base - ship.position).length() < ship.radius(),
                    "flame left behind"
                );
            }
        }
    }
    assert_eq!(s.flight, FlightMode::Space);
}

#[test]
fn pausing_freezes_the_crown_the_ring_and_the_flame_flicker() {
    let mut s = system();
    step(
        &mut s,
        &[SolarAction::TogglePause],
        &[],
        Look::default(),
        0.0,
    );
    let frozen = |s: &SolarScene| {
        s.effect_draws(W, H, 1)
            .iter()
            .filter(|d| !matches!(d.shader, EffectShader::Flame(_)))
            .map(|d| d.model.model_matrix.m12 + d.model.model_matrix.m13 + d.model.model_matrix.m14)
            .collect::<Vec<_>>()
    };
    let before = frozen(&s);
    let flame_time = |s: &SolarScene| {
        s.effect_draws(W, H, 1).iter().find_map(|d| match d.shader {
            EffectShader::Flame(f) => Some(f.time),
            _ => None,
        })
    };
    let flicker = flame_time(&s);
    for _ in 0..20 {
        step(&mut s, &[], &[], Look::default(), 0.05);
    }
    assert_eq!(frozen(&s), before);
    assert_eq!(flame_time(&s), flicker);
}

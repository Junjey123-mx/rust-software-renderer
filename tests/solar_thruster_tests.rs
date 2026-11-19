//! The spacecraft exhaust: one shared flame at each measured nozzle.

use raylib::math::{Matrix, Vector3};
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::io::obj::load_obj;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::flight::{
    FlightInput, FlightMode, THRUST_EASE, local_step, space_step,
};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::plasma_loops::PlasmaLoops;
use rust_software_renderer::solar::thrusters::{
    BOOST_STRETCH, FlameShader, IDLE_LENGTH, INSET, NOZZLES, POD_SHARE, THRUST_LENGTH, Thrusters,
    flame_length, flame_matrix, flame_mesh,
};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{CameraMode, ShipState, SolarMeshes, SolarScene};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

fn apply(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

fn close(a: Vector3, b: Vector3, tol: f32) -> bool {
    (a - b).length() < tol
}

fn is_closed(mesh: &Mesh) -> bool {
    let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
    for t in mesh.indices.chunks(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a, b)).or_default() += 1;
        }
    }
    edges
        .iter()
        .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
}

#[test]
fn every_nozzle_sits_on_a_square_mouth_of_the_ship_mesh() {
    let ship = load_obj(Path::new("assets/models/Laboratorio3.obj")).unwrap();
    for n in NOZZLES {
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
            let corner = n.center + Vector3::new(sx * n.half, sy * n.half, 0.0);
            assert!(
                ship.vertices.iter().any(|&v| close(v, corner, 1e-3)),
                "no mouth corner at {corner:?}"
            );
        }
        let reach = n.half + 0.1;
        let behind = ship.vertices.iter().filter(|v| {
            v.z > n.center.z + 1e-3 && (v.x - n.center.x).abs() < reach && v.y.abs() < reach
        });
        assert_eq!(
            behind.count(),
            0,
            "nothing covers the mouth at {:?}",
            n.center
        );
    }
    assert_eq!(NOZZLES[1].center.x, -NOZZLES[2].center.x, "symmetric pods");
}

#[test]
fn the_flame_is_a_closed_outward_solid_from_a_square_base_to_a_tip() {
    let mesh = flame_mesh();
    assert!(is_closed(&mesh));
    let interior = Vector3::new(0.0, 0.0, 0.3);
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (
            mesh.vertices[t[0]],
            mesh.vertices[t[1]],
            mesh.vertices[t[2]],
        );
        let n = (b - a).cross(c - a);
        assert!(n.dot((a + b + c) / 3.0 - interior) > 0.0, "inward face");
    }
    let base: Vec<_> = mesh.vertices.iter().filter(|v| v.z == 0.0).collect();
    assert_eq!(base.len(), 8);
    assert!(
        base.iter()
            .all(|v| (v.x.abs().max(v.y.abs()) - 1.0).abs() < 1e-5)
    );
    assert!(mesh.vertices.iter().all(|v| (0.0..=1.0).contains(&v.z)));
    assert!(mesh.vertices.contains(&Vector3::new(0.0, 0.0, 1.0)), "tip");
    assert_eq!(mesh.indices.len() / 3, 62);
}

#[test]
fn every_nozzle_shares_one_flame_mesh() {
    let t = Thrusters::default();
    assert!(Arc::ptr_eq(&t.mesh, &t.clone().mesh));
    assert_eq!(t.mesh.len(), flame_mesh().indices.len());
}

#[test]
fn to_world_matches_the_ship_model_matrix() {
    let mut s = ShipState::new(Vector3::new(1.0, -2.0, 3.0), 0.08);
    s.yaw(0.7);
    s.pitch(-0.4);
    let m = s.model_matrix();
    for p in [
        Vector3::zero(),
        Vector3::new(2.45, 0.3, 1.57),
        Vector3::new(-1.0, 2.0, -3.0),
    ] {
        assert!(close(apply(&m, p), s.to_world(p), 1e-5));
    }
}

#[test]
fn each_flame_rides_on_its_nozzle_and_points_out_of_the_back() {
    let mut s = ShipState::new(Vector3::new(4.0, 0.5, -2.0), 0.08);
    s.yaw(1.1);
    s.pitch(0.3);
    let length = 1.6;
    for n in NOZZLES {
        let m = flame_matrix(&s, &n, length);
        let base = s.to_world(n.center - Vector3::new(0.0, 0.0, INSET));
        assert!(close(apply(&m, Vector3::zero()), base, 1e-5));
        let tip = apply(&m, Vector3::new(0.0, 0.0, 1.0));
        let axis = tip - base;
        assert!(
            close(axis.normalized(), -s.forward, 1e-5),
            "out of the back"
        );
        assert!((axis.length() - (length + INSET) * s.scale).abs() < 1e-5);
        let corner = apply(&m, Vector3::new(1.0, 1.0, 0.0));
        let mouth = s.to_world(n.center + Vector3::new(n.half, n.half, -INSET));
        assert!(close(corner, mouth, 1e-5), "the base fills the mouth");
    }
    let mut moved = s;
    moved.position += Vector3::new(0.0, 1.0, 0.0);
    let (a, b) = (
        flame_matrix(&s, &NOZZLES[0], length),
        flame_matrix(&moved, &NOZZLES[0], length),
    );
    assert!((b.m13 - a.m13 - 1.0).abs() < 1e-5 && (b.m12 - a.m12).abs() < 1e-6);
}

const W: u32 = 320;
const H: u32 = 240;

/// The spacecraft alone at the origin, nose along -Z, seen by the
/// orthographic orbit camera from in front (`behind == false`, the Front
/// preset eye) or from behind.
fn ship_scene(behind: bool, thrust: f32) -> SolarScene {
    let mut camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    if behind {
        camera.azimuth += std::f32::consts::PI;
    }
    camera.elevation = 0.15;
    let view = OrbitView::for_world(camera, 0.45, Viewport::new(W, H, 0.1)).unwrap();
    let mut s = SolarScene::new(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    );
    s.camera_mode = CameraMode::Orbit;
    s.bodies.clear();
    s.plasma = PlasmaLoops::new(&[]);
    s.ship = ShipState::new(Vector3::zero(), 0.08);
    s.ship.thrust = thrust;
    s.center = Vector3::new(0.0, 5.0, -5.0);
    s
}

fn frame(s: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb
}

/// Pixels the flames change, and their colors.
fn flame_pixels(s: &SolarScene) -> Vec<Vector3> {
    let mut cold = s.clone();
    cold.thrusters = None;
    let (hot, cold) = (frame(s), frame(&cold));
    (0..H as i32)
        .flat_map(|y| (0..W as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| hot.get_depth(x, y) != cold.get_depth(x, y))
        .map(|(x, y)| hot.get_pixel(x, y).unwrap())
        .collect()
}

fn flames(s: &SolarScene) -> usize {
    s.effect_draws(W, H, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Flame(_)))
        .count()
}

#[test]
fn every_nozzle_keeps_an_idle_flame_and_cold_engines_draw_none() {
    let s = ship_scene(true, 0.0);
    assert_eq!(flames(&s), NOZZLES.len());
    let mut cold = s.clone();
    cold.thrusters = None;
    assert_eq!(flames(&cold), 0);
    let mut away = s.clone();
    away.view.camera.target = Vector3::new(30.0, 0.0, 0.0);
    assert_eq!(flames(&away), 0, "culled off screen");
}

#[test]
fn thrust_stretches_the_flames_and_the_pods_burn_shorter() {
    let mut s = ShipState::new(Vector3::zero(), 0.08);
    let idle: Vec<f32> = (0..3).map(|i| flame_length(&s, i, 0.0)).collect();
    s.thrust = 1.0;
    let full: Vec<f32> = (0..3).map(|i| flame_length(&s, i, 0.0)).collect();
    for i in 0..3 {
        assert!(
            full[i] > 3.0 * idle[i],
            "nozzle {i}: {} vs {}",
            full[i],
            idle[i]
        );
    }
    assert!(full[1] < full[0] && full[2] < full[0]);
    for step in 0..400 {
        let t = step as f32 * 0.013;
        for i in 0..3 {
            let share = if i == 0 { 1.0 } else { POD_SHARE };
            let mean = (IDLE_LENGTH + THRUST_LENGTH) * share;
            let l = flame_length(&s, i, t);
            assert!(
                (l / mean - 1.0).abs() <= 0.11 + 1e-5,
                "flicker {l} around {mean}"
            );
            assert_eq!(l, flame_length(&s, i, t), "same time, same flame");
        }
    }
    assert_ne!(
        flame_length(&s, 1, 0.3),
        flame_length(&s, 2, 0.3),
        "pods out of step"
    );
}

#[test]
fn the_engines_spool_up_and_down_without_jumps() {
    let mut s = ShipState::new(Vector3::zero(), 0.08);
    let fwd = FlightInput::from_actions(&[SolarAction::Forward]);
    space_step(&mut s, &fwd, 0.05);
    assert!((s.thrust - 0.05 / THRUST_EASE).abs() < 1e-6);
    for _ in 0..10 {
        space_step(&mut s, &fwd, 0.05);
    }
    assert_eq!(s.thrust, 1.0);
    space_step(&mut s, &FlightInput::default(), 0.05);
    assert!((s.thrust - (1.0 - 0.05 / THRUST_EASE)).abs() < 1e-6);
    let mut scene = ship_scene(true, 1.0);
    step(
        &mut scene,
        &[SolarAction::ToggleCamera],
        &[],
        Look::default(),
        0.0,
    );
    for _ in 0..6 {
        step(
            &mut scene,
            &[SolarAction::Forward],
            &[],
            Look::default(),
            0.05,
        );
    }
    assert_eq!(scene.ship.thrust, 0.0, "a parked ship idles");
}

#[test]
fn the_exhaust_glows_from_amber_to_red_and_never_blue() {
    let s = ShipState::new(Vector3::zero(), 0.08);
    let f = FlameShader::new(&s, 0, 1.5, 0.0);
    let eye = Vector3::new(0.0, 0.0, 1.0);
    let side_normal = Vector3::new(1.0, 0.0, 0.0);
    for k in 0..=20 {
        let z = k as f32 / 20.0;
        let c = f.shade(
            Vector3::new(0.5, 0.0, z),
            side_normal,
            Vector3::new(1.0, 0.0, 0.0),
        );
        assert!(c.x >= c.y && c.y >= c.z, "warm at {z}: {c:?}");
    }
    let near = f.shade(
        Vector3::new(0.9, 0.0, 0.05),
        side_normal,
        Vector3::new(1.0, 0.0, 0.0),
    );
    let tip = f.shade(
        Vector3::new(0.2, 0.0, 0.97),
        side_normal,
        Vector3::new(1.0, 0.0, 0.0),
    );
    assert!(near.y > 0.6 * near.x, "yellow at the nozzle: {near:?}");
    assert!(tip.x > 2.5 * tip.y, "red at the tip: {tip:?}");
    let facing = f.shade(
        Vector3::new(0.5, 0.0, 0.5),
        side_normal,
        Vector3::new(1.0, 0.0, 0.0),
    );
    let grazing = f.shade(Vector3::new(0.5, 0.0, 0.5), side_normal, eye);
    assert!(facing.x > grazing.x, "brightest facing the eye");
}

#[test]
fn the_exhaust_shows_orange_behind_the_ship_and_the_hull_hides_it_from_the_front() {
    let back = flame_pixels(&ship_scene(true, 1.0));
    let front = flame_pixels(&ship_scene(false, 1.0));
    let warm = |c: &&Vector3| c.x > 0.5 && c.x > c.z * 1.5 && c.y >= c.z;
    assert!(
        back.iter().filter(warm).count() > 150,
        "{} warm pixels",
        back.len()
    );
    assert!(
        front.len() * 4 < back.len(),
        "front {} vs back {}",
        front.len(),
        back.len()
    );
    let idle = flame_pixels(&ship_scene(true, 0.0));
    assert!(
        !idle.is_empty() && idle.len() < back.len(),
        "idle {} full {}",
        idle.len(),
        back.len()
    );
}

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

#[test]
fn the_afterburner_stretches_the_flames_as_the_boost_ramps() {
    let mut s = ShipState::new(Vector3::zero(), 0.08);
    s.thrust = 1.0;
    let at = |s: &ShipState| flame_length(s, 0, 0.4);
    let cruise = at(&s);
    let mut last = cruise;
    for k in 1..=10 {
        s.boost = k as f32 / 10.0;
        let l = at(&s);
        assert!(l > last, "longer at boost {}", s.boost);
        assert!(l - last < 0.2 * cruise, "no jump at boost {}", s.boost);
        last = l;
    }
    assert!((last / cruise - (1.0 + BOOST_STRETCH)).abs() < 1e-4);
}

#[test]
fn the_afterburner_turns_the_flame_blue_cyan_and_brighter() {
    let mut s = ShipState::new(Vector3::zero(), 0.08);
    let normal = Vector3::new(1.0, 0.0, 0.0);
    let shade = |s: &ShipState, z: f32| {
        FlameShader::new(s, 0, 1.5, 0.0).shade(Vector3::new(0.5, 0.0, z), normal, normal)
    };
    let warm: Vec<_> = (0..=10).map(|k| shade(&s, k as f32 / 10.0)).collect();
    s.boost = 0.5;
    let half: Vec<_> = (0..=10).map(|k| shade(&s, k as f32 / 10.0)).collect();
    s.boost = 1.0;
    let blue: Vec<_> = (0..=10).map(|k| shade(&s, k as f32 / 10.0)).collect();
    for k in 0..=10 {
        let (w, h, b) = (warm[k], half[k], blue[k]);
        assert!(b.z > b.x, "blue beats red at {k}: {b:?}");
        assert!(luminance(b) > luminance(w), "brighter at {k}");
        assert!(w.z < h.z && h.z < b.z, "blends in at {k}");
    }
    let (core, cyan) = (blue[0], blue[3]);
    assert!(core.x > 0.8 * core.z, "white-hot at the nozzle: {core:?}");
    assert!(
        cyan.y > 0.8 * cyan.z && cyan.y > 1.5 * cyan.x,
        "then cyan: {cyan:?}"
    );
}

#[test]
fn boosting_paints_the_exhaust_blue_on_screen() {
    let mut s = ship_scene(true, 1.0);
    let orange = flame_pixels(&s);
    s.ship.boost = 1.0;
    let boosted = flame_pixels(&s);
    let blue = |c: &&Vector3| c.z > c.x && c.z > 0.3;
    assert!(
        boosted.iter().filter(blue).count() * 2 > boosted.len(),
        "mostly blue"
    );
    assert_eq!(
        orange.iter().filter(blue).count(),
        0,
        "no blue without boost"
    );
    assert!(boosted.len() > orange.len(), "longer plumes");
}

#[test]
fn local_flight_keeps_the_exhaust_orange_even_with_the_space_bar_held() {
    let planet = Vector3::zero();
    let mut s = ShipState::new(Vector3::new(0.0, 0.0, 2.0), 0.08);
    let mut mode = FlightMode::Local {
        planet: 0,
        previous_planet_position: planet,
    };
    let held = FlightInput::from_actions(&[SolarAction::Forward, SolarAction::Boost]);
    for _ in 0..20 {
        local_step(&mut s, &mut mode, planet, 1.0, &held, 0.05);
    }
    assert_eq!(s.boost, 0.0);
    let f = FlameShader::new(&s, 0, 1.5, 0.0);
    let c = f.shade(
        Vector3::new(0.5, 0.0, 0.5),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    );
    assert!(c.x > c.z, "orange: {c:?}");
}

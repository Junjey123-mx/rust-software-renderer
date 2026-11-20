//! Project 3, close of P3-06 (C41-C52): the clover planet's surface
//! contract in the real scene. The seed moves the layout, the props turn
//! with the planet and stay on land, two model kinds (trees and rocks) and
//! four waterfalls are there, and the level of detail follows the distance.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::attach::props::{
    PropKind, PropPart, PropPose, ScatterRule, scatter,
};
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::clover::CloverParams;
use rust_software_renderer::solar::planet::{HeightField, PlanetMaterial};
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 400;
const H: u32 = 300;

fn spec_with_seed(seed: u32) -> SystemSpec {
    let mut spec = SystemSpec::project3();
    let p = spec
        .planets
        .iter_mut()
        .find(|p| p.id == PlanetId("clover"))
        .unwrap();
    p.seed = seed;
    p.material = PlanetMaterial::Clover(CloverParams {
        seed: seed as f32,
        ..CloverParams::default()
    });
    spec
}

fn build(spec: &SystemSpec) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
    .unwrap()
}

fn clover(s: &SolarScene) -> usize {
    s.body_index(PlanetId("clover")).unwrap()
}

fn field(s: &SolarScene) -> HeightField {
    s.relief_of(clover(s)).unwrap().field
}

/// The poses the scene baked (same rules and seeds as the registration).
fn poses(s: &SolarScene) -> Vec<PropPose> {
    let f = field(s);
    let HeightField::Clover(c) = f else {
        panic!("clover relief")
    };
    let mut p = scatter(&f, &ScatterRule::TREES, c.seed);
    p.extend(scatter(&f, &ScatterRule::ROCKS, c.seed ^ 0x5eed));
    p
}

#[test]
fn seed_controls_layout() {
    let a = poses(&build(&spec_with_seed(7)));
    let b = poses(&build(&spec_with_seed(8)));
    assert_eq!(
        a,
        poses(&build(&spec_with_seed(7))),
        "same seed, same poses"
    );
    // More than 90 % of seed 8's props stand where seed 7 has none.
    let moved = b
        .iter()
        .filter(|p| a.iter().all(|q| p.dir.dot(q.dir) < 0.9999))
        .count();
    assert!(moved * 10 > b.len() * 9, "{moved} of {}", b.len());
}

#[test]
fn props_follow_rotation() {
    let mut s = build(&SystemSpec::project3());
    let i = clover(&s);
    let p = s.bodies[i].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, 2.5 * s.radius_of(i));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    for t in [0.0f32, 10.0, 25.0] {
        s.clock.time = t;
        s.update();
        // Keep the planet in front of the ship as it moves.
        let p = s.bodies[i].transform.position;
        s.ship.position = p - Vector3::new(0.0, 0.0, 2.5 * s.radius_of(i));
        let host = s.bodies[i].transform.matrix();
        let draws = s.effect_draws(W, H, 1);
        let surface: Vec<_> = draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Prop(_) | EffectShader::Waterfall(_)))
            .collect();
        assert!(!surface.is_empty(), "t {t}");
        assert!(
            surface.iter().all(|d| d.model.model_matrix == host),
            "t {t}"
        );
    }
}

#[test]
fn no_props_under_sea() {
    let s = build(&SystemSpec::project3());
    let f = field(&s);
    let sea = f.sea_level().unwrap();
    let p = poses(&s);
    assert!(p.len() > 100, "{}", p.len());
    assert!(p.iter().all(|q| f.height(q.dir) > sea));
}

#[test]
fn four_waterfalls() {
    let s = build(&SystemSpec::project3());
    let falls = &s.waterfalls.as_ref().unwrap().falls;
    assert_eq!(falls.len(), 4);
    let f = field(&s);
    for w in falls {
        assert!(f.height(w.fall.top.normalized()) > f.sea_level().unwrap());
    }
}

#[test]
fn two_model_kinds_present() {
    let s = build(&SystemSpec::project3());
    let p = poses(&s);
    let trees = p.iter().filter(|q| q.kind == PropKind::Tree).count();
    let rocks = p.iter().filter(|q| q.kind == PropKind::Rock).count();
    assert!(trees > 50 && rocks > 20, "trees {trees}, rocks {rocks}");
    let props = s.props.as_ref().unwrap();
    for part in [PropPart::TreeTrunk, PropPart::TreeCanopy, PropPart::Rock] {
        let baked: usize = props
            .layers
            .iter()
            .filter(|l| l.part == part)
            .flat_map(|l| &l.sectors)
            .map(|sec| sec.props)
            .sum();
        assert!(baked > 0, "{part:?}");
    }
}

#[test]
fn lod_triangles_vary() {
    let config = SolarBenchConfig {
        width: W,
        height: H,
        threads: 2,
        ..SolarBenchConfig::default()
    };
    let options = ShadedOptions {
        threads: 2,
        ..ShadedOptions::default()
    };
    let body = |id| {
        let s = scenario_scene_on(id, &config, true).unwrap();
        let mut fb = Framebuffer::new(W, H);
        s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
            .unwrap();
        let i = clover(&s);
        *s.last_stats()
            .per_body
            .iter()
            .find(|b| b.index == i)
            .unwrap()
    };
    let (near, far) = (body("K"), body("K2"));
    assert!(near.lod > far.lod, "{near:?} {far:?}");
    assert!(near.triangles > 4 * far.triangles, "{near:?} {far:?}");
}

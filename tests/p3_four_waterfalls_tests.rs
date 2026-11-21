//! Project 3 clover waterfalls in the scene: one per mountain, each on its
//! own clover, turning with the planet, skipped past the horizon (but never
//! while any of it shows), and none when the parameter says so.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::attach::AttachmentKind;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::clover::CloverParams;
use rust_software_renderer::solar::planet::{HeightField, PlanetMaterial};
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 400;
const H: u32 = 300;

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

/// The ship `d` radii from the clover's center, looking at it.
fn looking_at_clover(d: f32) -> SolarScene {
    let mut s = build(&SystemSpec::project3());
    let i = clover(&s);
    let p = s.bodies[i].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(i));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

fn waterfall_draws(s: &SolarScene) -> usize {
    s.effect_draws(W, H, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Waterfall(_)))
        .count()
}

#[test]
fn four_distinct_waterfalls() {
    let s = build(&SystemSpec::project3());
    let falls = &s.waterfalls.as_ref().unwrap().falls;
    let mountains: Vec<usize> = falls.iter().map(|f| f.mountain).collect();
    assert_eq!(mountains, [0, 1, 2, 3]);
    for (a, fa) in falls.iter().enumerate() {
        for fb in &falls[a + 1..] {
            assert!((fa.fall.top - fb.fall.top).length() > 0.3);
        }
    }
    let kinds: Vec<_> = s.attachments.iter().map(|a| a.kind).collect();
    assert!(kinds.contains(&AttachmentKind::Waterfalls));
}

#[test]
fn each_attached_to_its_mountain() {
    let s = build(&SystemSpec::project3());
    let Some(HeightField::Clover(field)) = s.relief_of(clover(&s)).map(|r| r.field) else {
        panic!("the clover planet has its relief");
    };
    for f in &s.waterfalls.as_ref().unwrap().falls {
        let top = f.fall.top.normalized();
        assert!(field.inside(f.mountain, top) > 0.0, "top on its clover");
        let nearest = (0..4)
            .max_by(|&a, &b| {
                top.dot(field.centers[a])
                    .total_cmp(&top.dot(field.centers[b]))
            })
            .unwrap();
        assert_eq!(nearest, f.mountain);
        assert!(field.is_land(top) && !field.is_land(f.fall.bottom.normalized()));
    }
}

#[test]
fn rotate_with_host() {
    let mut s = looking_at_clover(2.5);
    let i = clover(&s);
    for yaw in [0.0, 1.3, 2.9] {
        s.bodies[i].transform.yaw = yaw;
        let host = s.bodies[i].transform.matrix();
        let draws = s.effect_draws(W, H, 1);
        let falls: Vec<_> = draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Waterfall(_)))
            .collect();
        assert!(!falls.is_empty(), "yaw {yaw}");
        assert!(falls.iter().all(|d| d.model.model_matrix == host));
    }
}

#[test]
fn culled_when_behind() {
    // From 2.5 radii some falls are on the far side and skipped. The four
    // classic falls plus the three monumental ones of the expansion.
    let s = looking_at_clover(2.5);
    let drawn = waterfall_draws(&s);
    assert!((1..7).contains(&drawn), "{drawn}");
    let mut all = s.clone();
    all.culling = false;
    assert_eq!(waterfall_draws(&all), 4 + 3);
    // Too small on screen: none.
    assert_eq!(waterfall_draws(&looking_at_clover(80.0)), 0);
    // Low over the sea (K3) the culled frame is the frame without culling:
    // nothing that shows is skipped.
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
    let mut k3 = scenario_scene_on("K3", &config, true).unwrap();
    let frame = |s: &SolarScene| {
        let mut fb = Framebuffer::new(W, H);
        s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
            .unwrap();
        fb.to_rgba_bytes()
    };
    let culled = frame(&k3);
    let fewer = k3.last_stats().draws;
    k3.culling = false;
    assert_eq!(culled, frame(&k3));
    assert!(fewer < k3.last_stats().draws, "culling skipped something");
}

#[test]
fn no_waterfalls_when_zero() {
    let mut spec = SystemSpec::project3();
    let planet = spec
        .planets
        .iter_mut()
        .find(|p| p.id == PlanetId("clover"))
        .unwrap();
    planet.material = PlanetMaterial::Clover(CloverParams {
        waterfalls: 0.0,
        ..CloverParams::default()
    });
    let s = build(&spec);
    assert!(s.waterfalls.is_none());
    assert!(
        s.attachments
            .iter()
            .all(|a| a.kind != AttachmentKind::Waterfalls)
    );
    let mut s = s;
    let i = clover(&s);
    let p = s.bodies[i].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, 2.5 * s.radius_of(i));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    assert_eq!(waterfall_draws(&s), 0);
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
}

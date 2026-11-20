//! Project 3 trail batching (C69): every trail of the planet in one buffer,
//! drawn once per group in view (the ships' ribbons, the cycles' walls)
//! instead of once per trail, the buffer reused frame after frame and the
//! image the same as drawing them one by one.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::attach::traffic::Traffic;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::system::spec::AttachmentSpec;
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 800;
const H: u32 = 600;

fn config() -> SolarBenchConfig {
    SolarBenchConfig {
        width: W,
        height: H,
        threads: 1,
        ..SolarBenchConfig::default()
    }
}

fn m(batched: bool) -> SolarScene {
    let mut s = scenario_scene_on("M", &config(), true).unwrap();
    s.traffic.as_mut().unwrap().batched = batched;
    s
}

/// (trail draws, trail triangles) of the frame.
fn trail_cost(s: &SolarScene) -> (usize, usize) {
    s.effect_draws(W, H, 1)
        .iter()
        .filter(|d| {
            matches!(
                d.shader,
                EffectShader::Trail(_) | EffectShader::TrailBatch(_)
            )
        })
        .fold((0, 0), |(n, t), d| (n + 1, t + d.mesh.len() / 3))
}

fn frame(s: &SolarScene) -> Vec<u8> {
    let mut fb = Framebuffer::new(W, H);
    let options = ShadedOptions {
        threads: 4,
        ..ShadedOptions::default()
    };
    s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    fb.to_rgba_bytes()
}

#[test]
fn one_draw_per_visible_group() {
    let (batched, separate) = (m(true), m(false));
    let traffic = separate.traffic.as_ref().unwrap();
    // Groups with a trail in view, from the trails drawn one by one.
    let draws = separate.effect_draws(W, H, 1);
    let groups = [0, 1]
        .iter()
        .filter(|&&g| {
            let (range, _) = traffic.group(g);
            draws.iter().any(|d| match &d.shader {
                EffectShader::Trail(t) => t.color == traffic.trail_color(range.start),
                _ => false,
            })
        })
        .count();
    let (n_sep, _) = trail_cost(&separate);
    let (n_batch, tris) = trail_cost(&batched);
    assert_eq!(groups, 2, "ribbons and walls in view in M");
    assert_eq!(n_batch, groups);
    assert!(n_sep > n_batch, "{n_sep} draws one by one");
    // The batch holds every trail of the planet, hidden or not.
    let all: usize = traffic.trails.iter().map(|t| t.len() / 3).sum();
    assert_eq!(tris, all);
    println!("trails in M: {n_sep} draws one by one, {n_batch} batched ({tris} triangles)");
}

#[test]
fn buffers_reused_no_alloc() {
    let mut s = m(true);
    let traffic = s.traffic.as_ref().unwrap();
    let total: usize = traffic.trails.iter().map(Vec::len).sum();
    assert_eq!(traffic.batch.len(), total);
    let (ptr, capacity) = (traffic.batch.as_ptr(), traffic.batch.capacity());
    for k in 0..30 {
        s.clock.time = 6.0 + k as f32 * 0.4;
        s.update();
        let traffic = s.traffic.as_ref().unwrap();
        assert_eq!(traffic.batch.len(), total);
        assert_eq!(
            (traffic.batch.as_ptr(), traffic.batch.capacity()),
            (ptr, capacity)
        );
        // The groups are the trails, in order.
        let (ships, cycles) = (traffic.group(0).1, traffic.group(1).1);
        assert_eq!(ships.len() + cycles.len(), total);
        assert_eq!(ships[..traffic.trails[0].len()], traffic.trails[0][..]);
        let last = traffic.trails.last().unwrap();
        assert_eq!(cycles[cycles.len() - last.len()..], last[..]);
    }
}

#[test]
fn no_z_artifacts_between_trails() {
    // The same pixels batched or one by one, where the trails cross each
    // other and the planet, and at another time.
    for t in [6.0, 9.5] {
        let (mut a, mut b) = (m(true), m(false));
        for s in [&mut a, &mut b] {
            s.clock.time = t;
            s.update();
        }
        let (fa, fb) = (frame(&a), frame(&b));
        let differ = fa
            .chunks(4)
            .zip(fb.chunks(4))
            .filter(|(p, q)| p != q)
            .count();
        assert_eq!(differ, 0, "t = {t}: {differ} pixels differ");
    }
}

#[test]
fn no_traffic_no_draws() {
    let mut spec = SystemSpec::project3();
    let hearts = spec
        .planets
        .iter_mut()
        .find(|p| p.id == PlanetId("hearts"))
        .unwrap();
    hearts.attachments = vec![AttachmentSpec::Traffic {
        ships: 0,
        cycles: 0,
        seed: 11,
    }];
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    let s = SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &spec,
    )
    .unwrap();
    assert!(s.traffic.is_none());
    assert_eq!(trail_cost(&s), (0, 0));
    // An empty traffic has empty groups.
    let meshes = SolarMeshes::load().unwrap();
    let field = HeightField::Hearts(HeartsField::new(11));
    let mut empty = Traffic::new(&meshes, field, 0, 11, 0.1, 1.3, None);
    empty.update_trails(3.0);
    assert!(empty.batch.is_empty());
    assert!(empty.group(0).1.is_empty() && empty.group(1).1.is_empty());
}

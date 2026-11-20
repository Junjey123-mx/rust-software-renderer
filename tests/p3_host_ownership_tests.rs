//! Project 3 host ownership: attachments ride on the body their id names,
//! following its translation and spin; a missing host draws nothing.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene;
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarScene;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::system::PlanetId;

const HOUSE: PlanetId = PlanetId("house");
const STAR: PlanetId = PlanetId("crown");

fn scene() -> SolarScene {
    let mut s = scenario_scene("D", &SolarBenchConfig::default()).unwrap();
    s.culling = false; // every card, wherever the camera looks
    s
}

/// Origins of the card draws (translation of their model matrices).
fn cards(s: &SolarScene) -> Vec<Vector3> {
    s.effect_draws(800, 600, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Card(_)))
        .map(|d| {
            let m = d.model.model_matrix;
            Vector3::new(m.m12, m.m13, m.m14)
        })
        .collect()
}

fn crown_matrix(s: &SolarScene) -> Option<raylib::math::Matrix> {
    s.effect_draws(800, 600, 1)
        .iter()
        .find(|d| matches!(d.shader, EffectShader::Crown(_)))
        .map(|d| d.model.model_matrix)
}

#[test]
fn host_pose_is_the_host_transform_and_radius() {
    let s = scene();
    let i = s.body_index(HOUSE).unwrap();
    let pose = s.host_pose(HOUSE).unwrap();
    assert_eq!(pose.transform, s.bodies[i].transform);
    assert_eq!(pose.radius, s.radius_of(i));
    assert_eq!(s.host_pose(PlanetId("moon")), None);
    assert_eq!(s.plasma_host(), s.body_index(STAR));
}

#[test]
fn attachments_follow_host_translation() {
    let s = scene();
    let i = s.body_index(HOUSE).unwrap();
    let before = cards(&s);
    let mut moved = s.clone();
    let shift = Vector3::new(0.7, -0.3, 1.1);
    moved.bodies[i].transform.position += shift;
    let after = cards(&moved);
    assert_eq!(before.len(), after.len());
    assert!(!before.is_empty());
    for (a, b) in before.iter().zip(&after) {
        assert!((*b - *a - shift).length() < 1e-4, "{a:?} -> {b:?}");
    }
}

#[test]
fn attachments_follow_host_spin() {
    let s = scene();
    let star = s.body_index(STAR).unwrap();
    let pose = s.host_pose(STAR).unwrap();
    let spin = s.crown.as_ref().unwrap().spin;
    // The crown is drawn with the host's transform plus its own turn.
    let expected = pose.attachment_transform(spin * s.clock.time).matrix();
    assert_eq!(crown_matrix(&s), Some(expected));
    // Turning the star turns the crown by the same angle.
    let mut turned = s.clone();
    turned.bodies[star].transform.yaw += 0.8;
    let expected = turned
        .host_pose(STAR)
        .unwrap()
        .attachment_transform(spin * s.clock.time)
        .matrix();
    assert_eq!(crown_matrix(&turned), Some(expected));
    assert_ne!(crown_matrix(&turned), crown_matrix(&s));
}

#[test]
fn missing_host_skips_draw_without_panic() {
    let mut s = scene();
    let star = s.body_index(STAR).unwrap();
    s.bodies.remove(star);
    assert_eq!(crown_matrix(&s), None);
    assert_eq!(s.plasma_host(), None);
    let i = s.body_index(HOUSE).unwrap();
    s.bodies.remove(i);
    assert!(cards(&s).is_empty());
    // The frame still renders (the ship alone).
    let mut fb = Framebuffer::new(160, 120);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    assert_eq!(s.last_stats().per_body.len(), 1);
}

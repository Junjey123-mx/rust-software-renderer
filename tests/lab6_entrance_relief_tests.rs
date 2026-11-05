//! Crimson Entrance: bump-mapped relief (E06) and view-dependent parallax (E07).

use raylib::math::Vector3;
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::celestial::{Body, CelestialScene, DebugView};
use rust_software_renderer::pipeline::{
    ShadedOptions, ShaderUniforms, ViewPreset, direction_from_angles,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::shading::lighting::lambert;
use rust_software_renderer::shading::sphere::{angular_distance, tangent_frame};

fn directions(n: usize) -> Vec<Vector3> {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).sqrt();
            let phi = golden * i as f32;
            Vector3::new(r * phi.cos(), y, r * phi.sin())
        })
        .collect()
}

fn uniforms(time: f32, eye: Vector3) -> ShaderUniforms {
    ShaderUniforms {
        time,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: eye,
    }
}

fn default_eye() -> Vector3 {
    direction_from_angles(-2.45, 0.55)
}

fn frame_at(params: EntranceParams, time: f32) -> Frame {
    prepare(&params, &uniforms(time, default_eye()), DebugView::None)
}

fn p() -> EntranceParams {
    EntranceParams::default()
}

fn render(params: &[(&str, f32)]) -> (Vec<u8>, Vec<Vector3>) {
    let mut scene =
        CelestialScene::new(Body::CrimsonEntrance, ViewPreset::Oblique, 160, 120).unwrap();
    scene.time = 8.0;
    let sets: Vec<(String, f32)> = params.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    scene.apply_overrides(&sets).unwrap();
    let mut fb = Framebuffer::new(160, 120);
    scene
        .render(
            &mut fb,
            TriangleMode::FilledScanline,
            ShadedOptions::default(),
        )
        .unwrap();
    (
        fb.to_rgba_bytes(),
        scene.vertices.iter().map(|v| v.position).collect(),
    )
}

fn differing_pixels(a: &[u8], b: &[u8]) -> usize {
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(x, y)| x != y)
        .count()
}

#[test]
fn zero_bump_restores_the_geometric_normal() {
    let off = frame_at(EntranceParams { bump: 0.0, ..p() }, 8.0);
    let flat = frame_at(
        EntranceParams {
            bump_strength: 0.0,
            ..p()
        },
        8.0,
    );
    for n in directions(1024) {
        assert_eq!(off.shading_normal(n), n);
        assert_eq!(flat.shading_normal(n), n);
    }
}

#[test]
fn bump_tilts_normals_where_height_varies() {
    let f = frame_at(p(), 8.0);
    let dirs = directions(4096);
    let tilted = dirs
        .iter()
        .filter(|&&n| angular_distance(n, f.shading_normal(n)) > 1.0_f32.to_radians())
        .count();
    assert!(tilted * 5 >= dirs.len(), "{tilted}");
}

#[test]
fn bumped_normals_are_unit_length() {
    let f = frame_at(p(), 8.0);
    for n in directions(4096) {
        assert!((f.shading_normal(n).length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn bump_changes_lighting_but_not_vertices() {
    let (with, verts_with) = render(&[("bump", 1.0)]);
    let (without, verts_without) = render(&[("bump", 0.0)]);
    assert_eq!(verts_with, verts_without);
    assert!(differing_pixels(&with, &without) > 1000);
}

#[test]
fn opposite_lights_reverse_bump_shading() {
    let f = frame_at(p(), 8.0);
    let (mut sxy, mut sxx, mut syy) = (0.0_f64, 0.0_f64, 0.0_f64);
    for n in directions(4096) {
        let (t, _) = tangent_frame(n);
        let (l1, l2) = ((n + t).normalized(), (n - t).normalized());
        let nb = f.shading_normal(n);
        let d1 = (lambert(nb, l1) - lambert(n, l1)) as f64;
        let d2 = (lambert(nb, l2) - lambert(n, l2)) as f64;
        sxy += d1 * d2;
        sxx += d1 * d1;
        syy += d2 * d2;
    }
    let correlation = sxy / (sxx * syy).sqrt();
    // Tilts towards one light turn away from the opposite one; the shared
    // (1 - cos tilt) term keeps the coefficient away from -1.
    assert!(correlation < -0.25, "{correlation}");
}

// ---- C21: view-dependent parallax (E07) ----

use rust_software_renderer::celestial::entrance::events::CRATER_SLOTS;
use rust_software_renderer::celestial::entrance::terrain;

fn frame_seen_from(params: EntranceParams, time: f32, eye: Vector3) -> Frame {
    prepare(&params, &uniforms(time, eye), DebugView::None)
}

/// Unit view direction at `angle` radians from `n`, tilted along `t`.
fn tilted(n: Vector3, t: Vector3, angle: f32) -> Vector3 {
    (n * angle.cos() + t * angle.sin()).normalized()
}

#[test]
fn parallax_is_identity_without_scale_or_at_normal_incidence() {
    let flat = frame_at(
        EntranceParams {
            parallax_scale: 0.0,
            ..p()
        },
        8.0,
    );
    let off = frame_at(
        EntranceParams {
            parallax: 0.0,
            ..p()
        },
        8.0,
    );
    for n in directions(512) {
        assert_eq!(flat.parallax_direction(n), n);
        assert_eq!(off.parallax_direction(n), n);
        let facing = frame_seen_from(p(), 8.0, n);
        assert!((facing.parallax_direction(n) - n).length() < 1e-5);
    }
}

#[test]
fn raised_spades_shift_toward_the_view_and_craters_away() {
    // Spade slot 0 at its apogee (t = 8) and crater slot 0 mature (t = 6).
    let spade_frame = frame_at(p(), 8.0);
    let spade = spade_frame.events[CRATER_SLOTS].center;
    let crater_frame = frame_at(p(), 6.0);
    let crater = crater_frame.events[0].center;
    assert!(terrain::macro_height(spade, &spade_frame) > 0.0);
    assert!(terrain::macro_height(crater, &crater_frame) < 0.0);
    for (n, time, toward) in [(spade, 8.0, true), (crater, 6.0, false)] {
        let (t, _) = tangent_frame(n);
        let v = tilted(n, t, std::f32::consts::FRAC_PI_4);
        let f = frame_seen_from(p(), time, v);
        let np = f.parallax_direction(n);
        assert!(
            angular_distance(n, np) > 0.1_f32.to_radians(),
            "shift too small"
        );
        let along = (np - n).dot(v - n * n.dot(v));
        assert_eq!(along > 0.0, toward, "shift direction at t = {time}");
    }
}

#[test]
fn parallax_fades_near_the_limb() {
    let f0 = frame_at(p(), 8.0);
    let n = f0.events[CRATER_SLOTS].center;
    let (t, _) = tangent_frame(n);
    // V . N = 0.05: grazing, the approximation is faded out completely.
    let grazing = frame_seen_from(p(), 8.0, tilted(n, t, 0.05_f32.acos()));
    assert!((grazing.parallax_direction(n) - n).length() < 1e-6);
    let steep = frame_seen_from(p(), 8.0, tilted(n, t, 0.8));
    assert!(angular_distance(n, steep.parallax_direction(n)) > 1e-4);
}

#[test]
fn parallax_offset_changes_with_the_camera_at_frozen_time() {
    let a = frame_seen_from(p(), 8.0, direction_from_angles(-2.45, 0.55));
    let b = frame_seen_from(p(), 8.0, direction_from_angles(-2.0, 0.55));
    let eye_a = direction_from_angles(-2.45, 0.55);
    let changed = directions(4096)
        .into_iter()
        .filter(|&n| n.dot(eye_a) > 0.3)
        .filter(|&n| angular_distance(a.parallax_direction(n), b.parallax_direction(n)) > 1e-4)
        .count();
    assert!(changed > 200, "{changed}");
}

#[test]
fn parallax_changes_the_rendered_relief() {
    let (with, verts_with) = render(&[("parallax", 1.0)]);
    let (without, verts_without) = render(&[("parallax", 0.0)]);
    assert_eq!(verts_with, verts_without);
    let pixels = with.len() / 4;
    assert!(
        differing_pixels(&with, &without) * 50 >= pixels,
        "at least 2 % of the image"
    );
}

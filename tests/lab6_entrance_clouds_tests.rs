//! Crimson Entrance: independent cloud layer (E04) and cloud shadows (E08).

use raylib::math::Vector3;
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::celestial::{Body, CelestialScene, DebugView};
use rust_software_renderer::pipeline::{
    ShadedOptions, ShaderUniforms, ViewPreset, direction_from_angles,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::shading::sphere::rotate_y;

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

fn uniforms(time: f32, light: Vector3) -> ShaderUniforms {
    ShaderUniforms {
        time,
        light_direction: light,
        eye_direction: direction_from_angles(-2.45, 0.55),
    }
}

fn default_light() -> Vector3 {
    direction_from_angles(-75.0_f32.to_radians(), 20.0_f32.to_radians())
}

fn frame_at(params: EntranceParams, time: f32) -> Frame {
    prepare(&params, &uniforms(time, default_light()), DebugView::None)
}

fn p() -> EntranceParams {
    EntranceParams::default()
}

fn mean_mask(f: &Frame, dirs: &[Vector3]) -> f32 {
    dirs.iter().map(|&n| f.clouds.mask(n)).sum::<f32>() / dirs.len() as f32
}

#[test]
fn cloud_mask_is_bounded() {
    let f = frame_at(p(), 3.0);
    for n in directions(4096) {
        let m = f.clouds.mask(n);
        assert!((0.0..=1.0).contains(&m), "{m}");
        assert!((0.0..=1.0).contains(&f.clouds.alpha(n)));
    }
}

#[test]
fn coverage_is_monotonic_and_zero_means_clear() {
    let dirs = directions(4096);
    let at = |coverage: f32| {
        mean_mask(
            &frame_at(
                EntranceParams {
                    cloud_coverage: coverage,
                    ..p()
                },
                2.0,
            ),
            &dirs,
        )
    };
    let (low, mid, high) = (at(0.2), at(0.45), at(0.7));
    assert!(low < mid && mid < high, "{low} {mid} {high}");
    assert_eq!(at(0.0), 0.0);
}

#[test]
fn clouds_move_while_terrain_stays_still() {
    let still = EntranceParams {
        events: 0.0,
        clouds: 0.0,
        ..p()
    };
    let dirs = directions(2048);
    let (a, b) = (frame_at(still, 0.0), frame_at(still, 5.0));
    for &n in &dirs {
        assert_eq!(a.surface_color(n), b.surface_color(n), "terrain is static");
    }
    let calm = EntranceParams { events: 0.0, ..p() };
    let (c, d) = (frame_at(calm, 0.0), frame_at(calm, 5.0));
    let moved = dirs
        .iter()
        .filter(|&&n| (c.clouds.mask(n) - d.clouds.mask(n)).abs() > 0.05)
        .count();
    assert!(
        moved * 5 >= dirs.len(),
        "only {moved} cloud samples changed"
    );
}

#[test]
fn cloud_layer_is_a_rigid_rotation_without_evolution() {
    let rigid = EntranceParams {
        cloud_evolution: 0.0,
        ..p()
    };
    let t = 7.0;
    let (now, start) = (frame_at(rigid, t), frame_at(rigid, 0.0));
    let angle = rigid.cloud_speed * t;
    for n in directions(2048) {
        let expected = start.clouds.mask(rotate_y(n, -angle));
        assert!((now.clouds.mask(n) - expected).abs() < 1e-5);
    }
}

#[test]
fn clouds_switch_matches_zero_opacity() {
    let off = frame_at(EntranceParams { clouds: 0.0, ..p() }, 4.0);
    let clear = frame_at(
        EntranceParams {
            cloud_opacity: 0.0,
            ..p()
        },
        4.0,
    );
    for n in directions(1024) {
        let s = rust_software_renderer::pipeline::SurfaceSample {
            x: 0,
            y: 0,
            depth: 0.0,
            object_position: n,
            normal: n,
            face_normal: n,
        };
        assert_eq!(off.shade(&s), clear.shade(&s));
    }
}

#[test]
fn clouds_add_no_geometry() {
    let mut counts = Vec::new();
    for clouds in [1.0, 0.0] {
        let mut scene =
            CelestialScene::new(Body::CrimsonEntrance, ViewPreset::Oblique, 160, 120).unwrap();
        scene.apply_overrides(&[("clouds".into(), clouds)]).unwrap();
        let mut fb = Framebuffer::new(160, 120);
        let stats = scene
            .render(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions::default(),
            )
            .unwrap();
        // 576 expanded corners of 192 triangles; the mesh keeps 98 vertices.
        assert_eq!(stats.vertices_processed, 576);
        assert_eq!(stats.triangles_processed, 192);
        counts.push(scene.vertices.len());
    }
    assert_eq!(counts, vec![576, 576]);
}

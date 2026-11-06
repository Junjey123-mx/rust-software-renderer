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
    // The switch removes the whole layer: invisible clouds and no shadows.
    let clear = frame_at(
        EntranceParams {
            cloud_opacity: 0.0,
            cloud_shadows: 0.0,
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

// ---- C17: light-aligned cloud shadows (E08) ----

use rust_software_renderer::pipeline::SurfaceSample;
use rust_software_renderer::shading::lighting::shell_exit;
use rust_software_renderer::shading::math::smoothstep;
use rust_software_renderer::shading::sphere::angular_distance;

fn surface(n: Vector3) -> SurfaceSample {
    SurfaceSample {
        x: 0,
        y: 0,
        depth: 0.0,
        object_position: n,
        normal: n,
        face_normal: n,
    }
}

#[test]
fn shadow_is_the_cloud_mask_sampled_toward_the_light() {
    let f = frame_at(p(), 6.0);
    let l = default_light();
    let mut shadowed = 0;
    for n in directions(4096) {
        let mu = n.dot(l);
        if mu <= 0.0 {
            continue;
        }
        let q = shell_exit(n, l, p().cloud_altitude);
        let expected = 1.0 - p().shadow_strength * f.clouds.mask(q) * smoothstep(0.0, 0.1, mu);
        assert_eq!(f.clouds.shadow(n, l), expected);
        if expected < 0.9 {
            shadowed += 1;
        }
    }
    assert!(shadowed > 100, "{shadowed}");
}

#[test]
fn zero_strength_or_switch_casts_no_shadow() {
    let l = default_light();
    for params in [
        EntranceParams {
            shadow_strength: 0.0,
            ..p()
        },
        EntranceParams {
            cloud_shadows: 0.0,
            ..p()
        },
    ] {
        let f = frame_at(params, 6.0);
        for n in directions(1024) {
            assert_eq!(f.clouds.shadow(n, l), 1.0);
        }
    }
}

#[test]
fn turning_the_light_moves_shadows_not_clouds() {
    let l1 = default_light();
    let l2 = direction_from_angles(-150.0_f32.to_radians(), 20.0_f32.to_radians());
    let a = prepare(&p(), &uniforms(6.0, l1), DebugView::None);
    let b = prepare(&p(), &uniforms(6.0, l2), DebugView::None);
    let mut moved = 0;
    let dirs = directions(4096);
    for &n in &dirs {
        assert_eq!(
            a.clouds.mask(n),
            b.clouds.mask(n),
            "clouds ignore the light"
        );
        if n.dot(l1) > 0.2
            && n.dot(l2) > 0.2
            && (a.clouds.shadow(n, l1) - b.clouds.shadow(n, l2)).abs() > 0.05
        {
            moved += 1;
        }
    }
    assert!(moved > 100, "{moved}");
}

#[test]
fn subsolar_shadow_has_no_offset_and_terminator_shadow_stretches() {
    let h = p().cloud_altitude;
    let l = default_light();
    assert!(
        (shell_exit(l, l, h) - l).length() < 1e-6,
        "q = n under the sun"
    );
    // A direction with n . l = 0.05, close to the terminator.
    let (t, _) = rust_software_renderer::shading::sphere::tangent_frame(l);
    let mu: f32 = 0.05;
    let n = (l * mu + t * (1.0 - mu * mu).sqrt()).normalized();
    let offset = angular_distance(n, shell_exit(n, l, h));
    assert!(offset > 0.25, "{offset}");
}

#[test]
fn night_side_is_not_shadowed() {
    let f = frame_at(p(), 6.0);
    let l = default_light();
    for n in directions(2048) {
        if n.dot(l) <= 0.0 {
            assert_eq!(f.clouds.shadow(n, l), 1.0);
        }
    }
}

#[test]
fn shadows_only_darken_direct_light() {
    let with = frame_at(p(), 6.0);
    let without = frame_at(
        EntranceParams {
            cloud_shadows: 0.0,
            ..p()
        },
        6.0,
    );
    let l = default_light();
    let mut darker = 0;
    for n in directions(4096) {
        let (a, b) = (with.surface_color(n), without.surface_color(n));
        assert!(a.x <= b.x + 1e-6 && a.y <= b.y + 1e-6 && a.z <= b.z + 1e-6);
        if n.dot(l) <= 0.0 {
            assert_eq!(a, b, "night side and emission unchanged");
        } else if (b - a).length() > 1e-3 {
            darker += 1;
        }
        assert!(with.shade(&surface(n)).x.is_finite());
    }
    assert!(darker > 100, "{darker}");
}

// ---- Clouds over open spade craters ----

use rust_software_renderer::celestial::entrance::CRATER_CLEARING;

#[test]
fn clouds_thin_softly_over_open_craters_only() {
    let f = frame_at(p(), 8.0);
    let e = f.events[0];
    assert!(e.alive() && e.stages.depth > 0.99);
    assert!(f.crater_clearing(e.center) > 0.95);
    // Soft: the layer keeps at least half of its opacity, and the edge of
    // the crater's reach is untouched.
    const { assert!(CRATER_CLEARING <= 0.5) };
    let far = directions(4096)
        .into_iter()
        .filter(|n| f.events.iter().all(|c| n.dot(c.center) < c.reach_cos))
        .collect::<Vec<_>>();
    assert!(far.len() > 2000);
    for n in far {
        assert_eq!(f.crater_clearing(n), 0.0);
    }
    let off = frame_at(EntranceParams { events: 0.0, ..p() }, 8.0);
    assert_eq!(off.crater_clearing(e.center), 0.0);
}

//! Crimson Entrance: regions classified from one height field.

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::entrance::terrain::{self, TerrainMaterial};
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::pipeline::{ShaderUniforms, SurfaceSample, direction_from_angles};
use std::collections::HashMap;

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

fn frame_with(params: EntranceParams, debug: DebugView) -> Frame {
    let u = ShaderUniforms {
        time: 0.0,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    };
    prepare(&params, &u, debug)
}

fn frame() -> Frame {
    frame_with(EntranceParams::default(), DebugView::None)
}

fn sample(n: Vector3) -> SurfaceSample {
    SurfaceSample {
        x: 0,
        y: 0,
        depth: 0.0,
        object_position: n * 0.85,
        normal: n,
        face_normal: n,
    }
}

#[test]
fn six_terrain_materials_cover_the_planet() {
    let f = frame();
    let dirs = directions(4096);
    let mut counts: HashMap<TerrainMaterial, usize> = HashMap::new();
    for &n in &dirs {
        *counts.entry(terrain::ground(n, &f).material).or_default() += 1;
    }
    for m in TerrainMaterial::ALL {
        assert!(counts.get(&m).copied().unwrap_or(0) > 0, "{m:?} missing");
    }
    let substantial = TerrainMaterial::ALL
        .iter()
        .filter(|m| counts[m] as f32 / dirs.len() as f32 >= 0.02)
        .count();
    assert!(substantial >= 5, "{counts:?}");
}

#[test]
fn material_colors_are_distinct() {
    let f = frame();
    let dirs = directions(512);
    let means: Vec<Vector3> = TerrainMaterial::ALL
        .iter()
        .map(|&m| {
            dirs.iter()
                .map(|&n| terrain::albedo(m, n, &f))
                .fold(Vector3::zero(), |a, c| a + c)
                / dirs.len() as f32
        })
        .collect();
    for i in 0..means.len() {
        for j in i + 1..means.len() {
            let d = (means[i] - means[j]).length();
            assert!(
                d >= 0.08,
                "{:?} vs {:?}: {d}",
                TerrainMaterial::ALL[i],
                TerrainMaterial::ALL[j]
            );
        }
    }
}

#[test]
fn height_is_finite_and_bounded() {
    let f = frame();
    for n in directions(4096) {
        let h = terrain::height(n, &f);
        assert!(h.is_finite() && h.abs() <= 2.0, "{h}");
        let m = terrain::macro_height(n, &f);
        assert!(m.is_finite() && m.abs() <= 2.0);
    }
}

#[test]
fn basin_is_a_deep_depression() {
    let f = frame();
    assert!(terrain::height(f.basin_center, &f) < -0.3);
    assert!(terrain::macro_height(f.basin_center, &f) < -0.3);
}

#[test]
fn terrain_is_deterministic_and_seeded() {
    let a = frame();
    let b = frame();
    let other = frame_with(
        EntranceParams {
            seed: 4242.0,
            ..EntranceParams::default()
        },
        DebugView::None,
    );
    let dirs = directions(256);
    let mut differs = 0;
    for &n in &dirs {
        assert_eq!(terrain::ground(n, &a), terrain::ground(n, &b));
        assert_eq!(a.shade(&sample(n)), b.shade(&sample(n)));
        if (terrain::height(n, &a) - terrain::height(n, &other)).abs() > 1e-3 {
            differs += 1;
        }
    }
    assert!(differs > 200, "{differs}");
}

#[test]
fn height_debug_view_is_grayscale() {
    let f = frame_with(EntranceParams::default(), DebugView::Height);
    let m = frame_with(EntranceParams::default(), DebugView::Materials);
    for n in directions(512) {
        let c = f.shade(&sample(n));
        assert!(c.x == c.y && c.y == c.z && (0.0..=1.0).contains(&c.x));
        let material = terrain::ground(n, &m).material;
        assert_eq!(m.shade(&sample(n)), material.debug_color());
    }
}

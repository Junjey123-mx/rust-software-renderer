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
        // The view shows the material looked up where the final image does
        // (the parallax-shifted direction since laboratory 6 adds parallax).
        let material = terrain::ground(m.parallax_direction(n), &m).material;
        assert_eq!(m.shade(&sample(n)), material.debug_color());
    }
}

// ---- Volcanic material hierarchy ----

use rust_software_renderer::shading::sdf::spade;
use rust_software_renderer::shading::sphere::from_local;

fn frame_at(time: f32) -> Frame {
    let u = ShaderUniforms {
        time,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    };
    prepare(&EntranceParams::default(), &u, DebugView::None)
}

#[test]
fn dark_volcanic_rock_dominates_the_visible_disk() {
    let f = frame_at(8.0);
    let eye = f.uniforms.eye_direction;
    let visible: Vec<Vector3> = directions(8192)
        .into_iter()
        .filter(|n| n.dot(eye) > 0.0)
        .collect();
    let share = |wanted: &[TerrainMaterial]| {
        visible
            .iter()
            .filter(|&&n| wanted.contains(&terrain::ground(n, &f).material))
            .count() as f32
            / visible.len() as f32
    };
    use TerrainMaterial::*;
    let rock = share(&[Obsidian, Basalt, Slate]);
    assert!(rock >= 0.6, "dark rock covers {rock}");
    assert!(share(&[Highland]) < rock / 3.0);
    assert!(share(&[Crack]) < 0.08, "fissures stay secondary");
    assert!(share(&[Lava]) < 0.05, "lava is reserved for the craters");
    for m in [Obsidian, Basalt, Slate, Highland, Ash] {
        assert!(!m.response().emissive, "{m:?}");
    }
    assert!(Crack.response().emissive && Lava.response().emissive);
}

#[test]
fn crimson_highlands_are_matte_rock_not_lava() {
    let f = frame_at(8.0);
    let highland = TerrainMaterial::Highland.response();
    assert_eq!((highland.specular, highland.wrap), (0.0, 0.0));
    // A typical molten floor at the peak (lava 0.6-1.0).
    let lava = f.lava_emission(0.8, 0.0);
    for n in directions(512) {
        let a = terrain::albedo(TerrainMaterial::Highland, n, &f);
        assert!(a.x <= 0.6 && a.x > 2.5 * a.y, "matte blood red {a:?}");
        // Even fully lit, the rock stays below half of the molten floor.
        assert!(a.x < 0.5 * lava.x, "{a:?} vs {lava:?}");
    }
}

#[test]
fn lava_only_exists_inside_open_spade_craters() {
    for time in [0.0, 6.5, 8.0, 15.0, 29.0] {
        let f = frame_at(time);
        for n in directions(8192) {
            if terrain::ground(n, &f).material == TerrainMaterial::Lava {
                let near = f
                    .events
                    .iter()
                    .any(|e| e.alive() && e.stages.glow > 0.0 && n.dot(e.center) >= e.reach_cos);
                assert!(near, "stray lava at t {time}");
            }
        }
    }
}

#[test]
fn crater_lips_read_as_ash() {
    // The large crater at full depth (tau 0.3).
    let f = frame_at(6.3);
    let e = f.events[0];
    let (mut lip, mut ash) = (0, 0);
    for i in 0..60 {
        for j in 0..60 {
            let (x, y) = (-0.9 + i as f32 * 0.03, -0.9 + j as f32 * 0.03);
            let s = spade((x, y));
            if (s - 0.07).abs() < 0.015 {
                lip += 1;
                let n = from_local(e.center, e.east, e.north, x * e.size, y * e.size);
                if terrain::ground(n, &f).material == TerrainMaterial::Ash {
                    ash += 1;
                }
            }
        }
    }
    assert!(
        lip > 30 && ash * 2 >= lip,
        "{ash} of {lip} lip points are ash"
    );
}

#[test]
fn shaded_colors_are_finite_and_bounded() {
    for time in [0.0, 3.0, 6.5, 8.0, 10.2, 40.0] {
        let f = frame_at(time);
        for n in directions(4096) {
            let c = f.shade(&sample(n));
            for v in [c.x, c.y, c.z] {
                assert!(v.is_finite() && (0.0..=4.0).contains(&v), "t {time}: {c:?}");
            }
        }
    }
}

//! Normal mapping in the software renderer (`solar::planet::normal_map`):
//! an RGB texture of tangent-space normals, baked from a height map, read
//! with bilinear filtering and taken to object space through the TBN frame,
//! on Clover Haven's rock at the Grand scale only.

use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::shading::sphere::tangent_frame;
use rust_software_renderer::solar::input::{PRESSED, SolarAction};
use rust_software_renderer::solar::planet::PlanetMaterial;
use rust_software_renderer::solar::planet::clover::{CloverParams, PreparedClover};
use rust_software_renderer::solar::planet::normal_map::{
    NormalMap, ROCK_RELIEF, ROCK_SIZE, decode, encode, perturb, rock, rock_height, sphere_uv,
};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::system::spec::GRAND_NORMAL_MAP;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};
use std::f32::consts::TAU;

fn near(a: Vector3, b: Vector3, eps: f32) -> bool {
    (a - b).length() < eps
}

#[test]
fn texels_store_normals_as_colors() {
    let up = Vector3::new(0.0, 0.0, 1.0);
    assert_eq!(encode(up), [128, 128, 255]);
    assert!(near(decode(encode(up)), up, 0.01));
    let tilted = Vector3::new(0.6, -0.48, 0.64);
    assert!(near(decode(encode(tilted)), tilted, 0.01));
}

#[test]
fn a_flat_height_map_bakes_the_flat_blue() {
    let map = NormalMap::from_height(16, 1.0, |_, _| 0.3);
    for (x, y) in [(0, 0), (5, 9), (15, 15)] {
        assert_eq!(map.texel(x, y), [128, 128, 255]);
    }
    assert!(near(
        map.sample(0.37, -2.1),
        Vector3::new(0.0, 0.0, 1.0),
        0.01
    ));
}

#[test]
fn the_bake_tilts_each_texel_away_from_the_slope() {
    // h = sin(2 pi u): rising at u = 0, falling at u = 1/2, flat along v.
    let size = 64;
    let map = NormalMap::from_height(size, 0.1, |u, _| (TAU * u).sin());
    let rising = decode(map.texel(0, 7)).normalized();
    let falling = decode(map.texel(size / 2, 7)).normalized();
    // Slope 2 pi times the relief 0.1: about 32 degrees.
    let tilt = (0.1 * TAU).atan();
    assert!(rising.x < 0.0 && falling.x > 0.0, "{rising:?} {falling:?}");
    assert!((rising.x.abs() - tilt.sin()).abs() < 0.02, "{rising:?}");
    assert!(rising.y.abs() < 0.01 && falling.y.abs() < 0.01);
}

#[test]
fn sampling_repeats_and_filters_between_texels() {
    let map = rock();
    let (u, v) = (0.3141, 0.7);
    assert!(near(map.sample(u, v), map.sample(u + 1.0, v - 3.0), 1e-4));
    // At a texel center the sample is that texel.
    let s = map.size() as f32;
    let center = map.sample((10.0 + 0.5) / s, (20.0 + 0.5) / s);
    assert!(near(center, decode(map.texel(10, 20)).normalized(), 1e-4));
    // Half way between two texels: their blend.
    let between = map.sample(11.0 / s, 20.5 / s);
    let blend = (decode(map.texel(10, 20)) + decode(map.texel(11, 20))).normalized();
    assert!(near(between, blend, 1e-3));
    for k in 0..200 {
        let n = map.sample(k as f32 * 0.0173, k as f32 * 0.031);
        assert!((n.length() - 1.0).abs() < 1e-4 && n.z > 0.0);
    }
}

#[test]
fn texture_coordinates_grow_east_and_north() {
    let n = Vector3::new(0.3, 0.4, 0.866).normalized();
    let (east, north) = tangent_frame(n);
    let (u, v) = sphere_uv(n, 1.0);
    let (ue, _) = sphere_uv((n + east * 1e-3).normalized(), 1.0);
    let (_, vn) = sphere_uv((n + north * 1e-3).normalized(), 1.0);
    assert!(ue > u && vn > v);
    // `tiles` repetitions around the equator, half as many pole to pole.
    let (u8, _) = sphere_uv(n, 8.0);
    assert!((u8 - 8.0 * u).abs() < 1e-5);
    let (_, south) = sphere_uv(Vector3::new(0.0, -1.0, 0.0), 8.0);
    let (_, north_pole) = sphere_uv(Vector3::new(0.0, 1.0, 0.0), 8.0);
    assert!(south.abs() < 1e-5 && (north_pole - 4.0).abs() < 1e-5);
    let (west, _) = sphere_uv(Vector3::new(-1e-4, 0.0, -1.0).normalized(), 8.0);
    let (east_edge, _) = sphere_uv(Vector3::new(1e-4, 0.0, -1.0).normalized(), 8.0);
    assert!((west - east_edge).abs() > 7.99, "one turn spans 8 tiles");
}

#[test]
fn the_tbn_frame_takes_the_texel_to_the_surface() {
    let n = Vector3::new(0.2, 0.5, 0.84).normalized();
    let (east, north) = tangent_frame(n);
    let up = Vector3::new(0.0, 0.0, 1.0);
    // The flat texel and strength 0 keep the normal.
    assert!(near(perturb(n, n, up, 1.0), n, 1e-5));
    let leaning = Vector3::new(0.5, 0.0, 0.866);
    assert!(near(perturb(n, n, leaning, 0.0), n, 1e-6));
    // +x leans east, +y north, by the texel's own angle.
    let e = perturb(n, n, leaning, 1.0);
    assert!(e.dot(east) > 0.49 && e.dot(north).abs() < 1e-4 && (e.length() - 1.0).abs() < 1e-5);
    let nn = perturb(n, n, Vector3::new(0.0, 0.5, 0.866), 1.0);
    assert!(nn.dot(north) > 0.49 && nn.dot(east).abs() < 1e-4);
    // Around a relief normal that is not radial the frame follows it.
    let relief = (n + east * 0.4).normalized();
    assert!(near(perturb(n, relief, up, 1.0), relief, 1e-5));
    let r = perturb(n, relief, leaning, 1.0);
    assert!((r.dot(relief) - 0.866).abs() < 1e-3, "{r:?}");
}

#[test]
fn the_rock_map_is_seeded_seamless_and_varied() {
    let again = NormalMap::from_height(ROCK_SIZE, ROCK_RELIEF, rock_height);
    assert_eq!(&again, rock());
    for k in 0..32 {
        let t = k as f32 / 32.0;
        assert!((rock_height(0.0, t) - rock_height(1.0, t)).abs() < 1e-4);
        assert!((rock_height(t, 0.0) - rock_height(t, 1.0)).abs() < 1e-4);
    }
    let map = rock();
    let mut tilts: Vec<f32> = (0..ROCK_SIZE * ROCK_SIZE)
        .map(|k| {
            let n = decode(map.texel(k % ROCK_SIZE, k / ROCK_SIZE)).normalized();
            n.z.acos().to_degrees()
        })
        .collect();
    tilts.sort_by(f32::total_cmp);
    let (p50, max) = (tilts[tilts.len() / 2], tilts[tilts.len() - 1]);
    // Plates with grain, cracks steeper; nothing faces sideways.
    assert!(
        (8.0..30.0).contains(&p50) && (40.0..80.0).contains(&max),
        "{p50} {max}"
    );
}

fn clover(normal_map: f32) -> PreparedClover {
    let params = CloverParams {
        normal_map,
        ..CloverParams::default()
    };
    PreparedClover {
        field: params.field(),
        params,
        light: Vector3::new(0.3, 0.8, 0.52).normalized(),
        time: 2.0,
    }
}

#[test]
fn clover_rock_is_lit_through_the_map_and_off_changes_nothing() {
    let (off, on) = (clover(0.0), clover(1.0));
    let (mut lit, mut changed) = (0, 0);
    for k in 0..2000 {
        let a = k as f32 * 0.731;
        let n = Vector3::new(
            a.cos() * (a * 0.37).cos(),
            (a * 0.37).sin(),
            a.sin() * (a * 0.37).cos(),
        )
        .normalized();
        let eye = (n + Vector3::new(0.1, 0.2, 0.0)).normalized();
        let h = off.field.height(n);
        let base = off.shade(n, eye);
        let mapped = on.shade(n, eye);
        // Off: the relief normal bit for bit.
        assert_eq!(off.mapped_normal(n, n, h), n);
        if h <= off.field.sea {
            assert_eq!(base, mapped, "the sea has no rock");
            continue;
        }
        // The night side only sees the ambient light.
        if n.dot(off.light) < 0.2 {
            continue;
        }
        lit += 1;
        if (base - mapped).length() > 1e-3 {
            changed += 1;
        }
    }
    // Nearly flat texels on grass (a fifth of the strength) barely move.
    assert!(lit > 30 && changed * 3 > lit * 2, "{changed} of {lit}");
}

#[test]
fn only_the_grand_clover_maps_rock_and_b_toggles_it() {
    let strength = |spec: &SystemSpec| {
        spec.planets.iter().find_map(|p| match p.material {
            PlanetMaterial::Clover(c) => Some(c.normal_map),
            _ => None,
        })
    };
    assert_eq!(strength(&SystemSpec::project3()), Some(0.0));
    assert_eq!(strength(&SystemSpec::grand()), Some(GRAND_NORMAL_MAP));
    assert!(
        CloverParams::SPECS
            .iter()
            .any(|s| s.name == "normal_map" && s.max >= 1.0)
    );
    assert!(PRESSED.contains(&(KeyboardKey::KEY_B, SolarAction::ToggleNormalMap)));
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let light = Vector3::new(0.0, 1.0, 0.0);
    let mut grand = SolarScene::grand_system(SolarMeshes::load().unwrap(), view, light).unwrap();
    let current = |s: &SolarScene| {
        s.bodies.iter().find_map(|b| match b.surface {
            Some(PlanetMaterial::Clover(c)) => Some(c.normal_map),
            _ => None,
        })
    };
    assert_eq!(current(&grand), Some(GRAND_NORMAL_MAP));
    assert!(!grand.toggle_normal_map());
    assert_eq!(current(&grand), Some(0.0));
    assert!(grand.toggle_normal_map());
    assert_eq!(current(&grand), Some(GRAND_NORMAL_MAP));
}

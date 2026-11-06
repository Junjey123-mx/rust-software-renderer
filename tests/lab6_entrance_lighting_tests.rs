//! Crimson Entrance: material light responses (E03) and night glow (E02).

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::entrance::terrain::TerrainMaterial;
use rust_software_renderer::celestial::entrance::{EntranceParams, Frame, prepare};
use rust_software_renderer::pipeline::{ShaderUniforms, direction_from_angles};

fn unit(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z).normalized()
}

fn frame(params: EntranceParams) -> Frame {
    let u = ShaderUniforms {
        time: 8.0,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    };
    prepare(&params, &u, DebugView::None)
}

fn p() -> EntranceParams {
    EntranceParams::default()
}

const ALBEDO: Vector3 = Vector3 {
    x: 0.4,
    y: 0.1,
    z: 0.12,
};

#[test]
fn obsidian_highlights_depend_on_the_view() {
    let f = frame(p());
    let n = unit(0.1, 0.3, 1.0);
    let l = unit(0.6, 0.4, 0.7);
    let mirror = n * (2.0 * n.dot(l)) - l;
    let far = unit(-0.8, -0.1, 0.6);
    let peak = f.reflect_light(TerrainMaterial::Obsidian, ALBEDO, n, l, mirror, 1.0);
    let dull = f.reflect_light(TerrainMaterial::Obsidian, ALBEDO, n, l, far, 1.0);
    assert!(peak.x > dull.x + 0.5, "{peak:?} vs {dull:?}");
}

#[test]
fn rock_is_view_independent() {
    let f = frame(p());
    let n = unit(0.1, 0.3, 1.0);
    let l = unit(0.6, 0.4, 0.7);
    for material in [
        TerrainMaterial::Basalt,
        TerrainMaterial::Highland,
        TerrainMaterial::Slate,
        TerrainMaterial::Ash,
    ] {
        let a = f.reflect_light(material, ALBEDO, n, l, unit(0.0, 0.0, 1.0), 1.0);
        let b = f.reflect_light(material, ALBEDO, n, l, unit(-0.9, 0.2, 0.3), 1.0);
        assert_eq!(a, b, "{material:?}");
    }
}

#[test]
fn fissures_ignore_the_light_direction() {
    let f = frame(p());
    let n = unit(0.1, 0.3, 1.0);
    let v = unit(0.0, 0.0, 1.0);
    let lit = f.reflect_light(
        TerrainMaterial::Crack,
        ALBEDO,
        n,
        unit(0.6, 0.4, 0.7),
        v,
        1.0,
    );
    let dark = f.reflect_light(
        TerrainMaterial::Crack,
        ALBEDO,
        n,
        unit(-0.6, -0.4, -0.7),
        v,
        1.0,
    );
    assert_eq!(lit, dark);
}

#[test]
fn specular_switch_makes_obsidian_matte() {
    let f = frame(EntranceParams {
        specular: 0.0,
        ..p()
    });
    let n = unit(0.1, 0.3, 1.0);
    let l = unit(0.6, 0.4, 0.7);
    let mirror = n * (2.0 * n.dot(l)) - l;
    let a = f.reflect_light(TerrainMaterial::Obsidian, ALBEDO, n, l, mirror, 1.0);
    let b = f.reflect_light(
        TerrainMaterial::Obsidian,
        ALBEDO,
        n,
        l,
        unit(-0.8, -0.1, 0.6),
        1.0,
    );
    assert_eq!(a, b);
    let basalt = f.reflect_light(TerrainMaterial::Basalt, ALBEDO, n, l, mirror, 1.0);
    assert_eq!(a, basalt, "matte obsidian reflects like rock");
}

// ---- C20: glowing fissures on the night side (E02) ----

/// Normal at a chosen angle to the light: `n . l = cos_angle`.
fn normal_with(l: Vector3, n_dot_l: f32) -> Vector3 {
    let (t, _) = rust_software_renderer::shading::sphere::tangent_frame(l);
    (l * n_dot_l + t * (1.0 - n_dot_l * n_dot_l).sqrt()).normalized()
}

#[test]
fn fissures_glow_brighter_at_night() {
    let f = frame(p());
    let l = unit(0.6, 0.4, 0.7);
    let night = f.emission(1.0, normal_with(l, -0.5), l);
    let day = f.emission(1.0, normal_with(l, 0.5), l);
    assert!(
        night.x >= 3.0 * day.x && day.x > 0.0,
        "{night:?} vs {day:?}"
    );
    let off = frame(EntranceParams {
        night_glow: 0.0,
        ..p()
    });
    assert_eq!(off.emission(1.0, normal_with(l, -0.9), l), Vector3::zero());
}

#[test]
fn night_glow_is_monotonic_across_the_terminator() {
    let f = frame(p());
    let l = unit(0.6, 0.4, 0.7);
    let mut previous = -1.0;
    for i in 0..=200 {
        let n_dot_l = 1.0 - 2.0 * i as f32 / 200.0;
        let factor = f.night_factor(normal_with(l, n_dot_l), l);
        assert!((0.0..=1.0).contains(&factor));
        assert!(factor >= previous - 1e-6, "not monotonic at {n_dot_l}");
        previous = factor;
    }
    assert_eq!(f.night_factor(normal_with(l, 0.5), l), 0.0);
    assert_eq!(f.night_factor(normal_with(l, -0.5), l), 1.0);
}

#[test]
fn night_lights_switch_keeps_day_level() {
    let off = frame(EntranceParams {
        night_lights: 0.0,
        ..p()
    });
    let on = frame(p());
    let l = unit(0.6, 0.4, 0.7);
    let day = on.emission(1.0, normal_with(l, 0.7), l);
    for n_dot_l in [0.7, 0.0, -0.4, -0.95] {
        assert_eq!(off.emission(1.0, normal_with(l, n_dot_l), l), day);
    }
}

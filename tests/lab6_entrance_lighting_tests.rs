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

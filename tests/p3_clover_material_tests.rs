//! Project 3 clover material: land and sea shaded apart, rock on steep
//! slopes, parameters through the `params!` pattern and a dark night side.
//! A shader of its own, not a recoloring of a laboratory 6 body.

use raylib::math::Vector3;
use rust_software_renderer::solar::planet::PlanetMaterial;
use rust_software_renderer::solar::planet::clover::{CloverParams, PreparedClover};

fn prepared(light: Vector3) -> PreparedClover {
    let params = CloverParams::default();
    PreparedClover {
        field: params.field(),
        params,
        light: light.normalized(),
        time: 0.0,
    }
}

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

#[test]
fn land_and_water_differ() {
    let p = prepared(Vector3::new(0.0, 1.0, 0.0));
    let land = p.field.centers[0];
    let sea = -land;
    assert!(p.field.is_land(land) && !p.field.is_land(sea));
    // Both lit from straight above.
    let lit = |n: Vector3| prepared(n).shade(n * 0.9, n);
    let (g, w) = (lit(land), lit(sea));
    assert!((g - w).length() > 0.2, "{g:?} {w:?}");
    assert!(w.z > w.x && w.z > 0.1, "the sea is blue: {w:?}");
    // Water shows a star glint where the light mirrors to the eye.
    let n = sea;
    let side = n.cross(Vector3::new(0.3, 1.0, 0.2)).normalized();
    let l = (n + side * 0.5).normalized();
    let e = (n - side * 0.5).normalized();
    let glint = prepared(l).shade(n, e);
    let matte = prepared(l).shade(n, (n + side * 0.5).normalized());
    assert!(luminance(glint) > luminance(matte) + 0.3);
}

#[test]
fn slope_turns_to_rock() {
    let p = prepared(Vector3::new(0.0, 1.0, 0.0));
    let n = Vector3::new(0.0, 1.0, 0.0);
    let h = p.field.sea + 0.02;
    let flat = p.land_albedo(n, n, h);
    let steep_normal = Vector3::new(0.8, 0.6, 0.0);
    let steep = p.land_albedo(n, steep_normal, h);
    let rock = Vector3::new(0x7D as f32, 0x72 as f32, 0x66 as f32) / 255.0;
    assert!((steep - rock).length() < 0.02, "{steep:?}");
    assert!(flat.y > flat.x, "gentle ground stays green: {flat:?}");
    // With the threshold at its maximum the same slope keeps its grass.
    let mut grassy = p;
    grassy.params.slope_rock = 1.0;
    assert!((grassy.land_albedo(n, steep_normal, h) - rock).length() > 0.1);
}

#[test]
fn params_override() {
    let mut params = CloverParams::default();
    params.set("bump", 0.0).unwrap();
    params.set("seed", 42.0).unwrap();
    assert_eq!(params.get("bump"), Some(0.0));
    assert_eq!(params.field().seed, 42);
    assert!(params.set("nope", 1.0).is_err(), "unknown parameter");
    assert!(params.set("bump", 2.0).is_err(), "out of range");
    assert_eq!(params.get("bump"), Some(0.0));
    assert!(CloverParams::SPECS.iter().any(|s| s.name == "water_gloss"));
    assert_ne!(
        PlanetMaterial::Clover(params),
        PlanetMaterial::Clover(CloverParams::default())
    );
}

#[test]
fn day_night_contrast() {
    let p = prepared(Vector3::new(0.0, 1.0, 0.0));
    for n in [p.field.centers[1], -p.field.centers[1]] {
        let day = prepared(n).shade(n, n);
        let night = prepared(-n).shade(n, n);
        assert!(
            luminance(day) > 8.0 * luminance(night),
            "{day:?} vs {night:?}"
        );
        assert!(luminance(night) > 0.0, "the night side is dim, not void");
    }
}

/// `P3_DUMP_DIR=<dir>` writes the clover material on an orthographic disc
/// (a clover facing the eye, the star to the upper left), to look at it.
#[test]
fn clover_material_dump() {
    let Some(dir) = std::env::var_os("P3_DUMP_DIR") else {
        return;
    };
    let size = 320usize;
    let base = prepared(Vector3::new(-0.5, 0.6, 0.62));
    let c = base.field.centers[0];
    let right = c.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let up = right.cross(c);
    let to_obj = |v: Vector3| right * v.x + up * v.y + c * v.z;
    let p = PreparedClover {
        light: to_obj(Vector3::new(-0.5, 0.6, 0.62)).normalized(),
        ..base
    };
    let mut rgb = Vec::with_capacity(size * size * 3);
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let v = 1.0 - (y as f32 + 0.5) / size as f32 * 2.0;
            let r2 = u * u + v * v;
            let color = if r2 < 1.0 {
                let n = to_obj(Vector3::new(u, v, (1.0 - r2).sqrt()));
                p.shade(n, c)
            } else {
                Vector3::zero()
            };
            for ch in [color.x, color.y, color.z] {
                rgb.push((ch.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    let header = format!("P6 {size} {size} 255\n");
    let path = std::path::Path::new(&dir).join("p3_clover_material.ppm");
    std::fs::write(path, [header.as_bytes(), &rgb].concat()).unwrap();
}

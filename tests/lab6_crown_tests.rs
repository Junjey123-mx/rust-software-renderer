//! The King's Crown: emissive royal plasma.

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::crown::{CrownParams, Frame, prepare};
use rust_software_renderer::pipeline::{ShaderUniforms, SurfaceSample, direction_from_angles};

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

fn uniforms(time: f32) -> ShaderUniforms {
    ShaderUniforms {
        time,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    }
}

fn frame(params: CrownParams, time: f32) -> Frame {
    prepare(&params, &uniforms(time), DebugView::None)
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

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

#[test]
fn crown_colors_are_finite_and_bounded() {
    let f = frame(CrownParams::default(), 3.0);
    for n in directions(4096) {
        let c = f.shade(&sample(n));
        for v in [c.x, c.y, c.z] {
            assert!(v.is_finite() && (0.0..=2.0).contains(&v));
        }
    }
}

#[test]
fn crown_is_emissive_and_ignores_the_light() {
    let p = CrownParams::default();
    let a = prepare(&p, &uniforms(2.0), DebugView::None);
    let mut other = uniforms(2.0);
    other.light_direction = -other.light_direction;
    let b = prepare(&p, &other, DebugView::None);
    for n in directions(512) {
        assert_eq!(a.shade(&sample(n)), b.shade(&sample(n)));
    }
}

#[test]
fn crown_palette_is_warm() {
    let f = frame(CrownParams::default(), 0.0);
    let dirs = directions(4096);
    let (r, b) = dirs.iter().fold((0.0, 0.0), |(r, b), &n| {
        let c = f.shade(&sample(n));
        (r + c.x, b + c.z)
    });
    let count = dirs.len() as f32;
    assert!(
        r / count > b / count + 0.3,
        "r {} b {}",
        r / count,
        b / count
    );
}

#[test]
fn crown_has_currents_and_veins() {
    let f = frame(CrownParams::default(), 0.0);
    let dirs = directions(4096);
    let lum: Vec<f32> = dirs
        .iter()
        .map(|&n| luminance(f.shade(&sample(n))))
        .collect();
    let mean = lum.iter().sum::<f32>() / lum.len() as f32;
    let std = (lum.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / lum.len() as f32).sqrt();
    assert!(std > 0.08, "luminance deviation {std}");
    let hot = dirs.iter().filter(|&&n| f.temperature(n) > 0.85).count() as f32 / dirs.len() as f32;
    assert!((0.02..=0.40).contains(&hot), "hot fraction {hot}");
}

#[test]
fn crown_is_deterministic_and_seeded() {
    let a = frame(CrownParams::default(), 1.5);
    let b = frame(CrownParams::default(), 1.5);
    let other = frame(
        CrownParams {
            seed: 99.0,
            ..CrownParams::default()
        },
        1.5,
    );
    let dirs = directions(256);
    let mut differs = 0;
    for &n in &dirs {
        assert_eq!(a.shade(&sample(n)), b.shade(&sample(n)));
        if (a.temperature(n) - other.temperature(n)).abs() > 1e-3 {
            differs += 1;
        }
    }
    assert!(differs > 200, "{differs}");
}

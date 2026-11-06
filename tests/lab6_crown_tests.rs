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

// ---- C11: corona, sunspots, diadem and eclipse (E01 material animation) ----

use rust_software_renderer::celestial::crown::eclipse_strength;
use rust_software_renderer::celestial::{Body, CelestialScene};
use rust_software_renderer::pipeline::{ShadedOptions, ViewPreset};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;

fn render_crown(time: f32) -> (Vec<u8>, Vec<Vector3>, usize) {
    let mut scene = CelestialScene::new(Body::KingsCrown, ViewPreset::Oblique, 200, 150).unwrap();
    scene.time = time;
    let mut fb = Framebuffer::new(200, 150);
    let stats = scene
        .render(
            &mut fb,
            TriangleMode::FilledScanline,
            ShadedOptions::default(),
        )
        .unwrap();
    let positions = scene.vertices.iter().map(|v| v.position).collect();
    (fb.to_rgba_bytes(), positions, stats.triangles_processed)
}

#[test]
fn crown_material_changes_with_fixed_camera_and_geometry() {
    let (a, pos_a, tris_a) = render_crown(0.0);
    let (b, pos_b, tris_b) = render_crown(2.0);
    assert_eq!(pos_a, pos_b, "vertices never move");
    assert_eq!(tris_a, tris_b);
    let pixels = a.len() / 4;
    let changed = a
        .chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(p, q)| (0..3).any(|i| p[i].abs_diff(q[i]) > 2))
        .count();
    let disk = a.chunks_exact(4).filter(|p| p[..3] != [0, 0, 0]).count();
    assert!(disk > pixels / 4);
    assert!(
        changed * 5 >= disk,
        "only {changed} of {disk} disk pixels changed"
    );
}

#[test]
fn crown_change_is_not_a_rigid_rotation() {
    let dirs = directions(4096);
    let histogram = |time: f32| {
        let f = frame(CrownParams::default(), time);
        let mut bins = [0.0_f32; 8];
        for &n in &dirs {
            let l = luminance(f.shade(&sample(n))).clamp(0.0, 0.999);
            bins[(l * 8.0) as usize] += 1.0 / dirs.len() as f32;
        }
        bins
    };
    let (h0, h1) = (histogram(0.0), histogram(4.5));
    let tv: f32 = h0.iter().zip(&h1).map(|(a, b)| (a - b).abs()).sum::<f32>() * 0.5;
    assert!(
        tv >= 0.05,
        "total variation {tv}: a rigid rotation would keep it near 0"
    );
}

#[test]
fn sunspot_area_breathes() {
    let dirs = directions(16_384);
    let p = CrownParams::default();
    // Dark area of spot 0 alone: penumbra directions closest to its center.
    let dark = |time: f32| {
        let f = frame(p, time);
        let c = f.spot_centers[0];
        dirs.iter()
            .filter(|&&n| n.dot(c) > (1.2 * p.spot_radius).cos() && f.spots(n).1 > 0.5)
            .count() as f32
    };
    let (big, small) = (
        dark(p.spot_breath_period * 0.25),
        dark(p.spot_breath_period * 0.75),
    );
    assert!(
        (big - small).abs() / big.max(small) >= 0.15,
        "spot area {big} vs {small}"
    );
}

#[test]
fn corona_pulse_widens_the_limb() {
    let p = CrownParams::default();
    let eye = uniforms(0.0).eye_direction;
    let visible: Vec<Vector3> = directions(8192)
        .into_iter()
        .filter(|n| n.dot(eye) > 0.0)
        .collect();
    let mean_corona = |time: f32| {
        let f = frame(p, time);
        visible.iter().map(|&n| f.corona(n)).sum::<f32>() / visible.len() as f32
    };
    let wide = mean_corona(p.corona_pulse_period * 0.25);
    let narrow = mean_corona(p.corona_pulse_period * 0.75);
    assert!(wide > narrow * 1.2, "corona {wide} vs {narrow}");
}

#[test]
fn eclipse_darkens_the_core_but_not_the_corona() {
    let p = CrownParams::default();
    let t = p.eclipse_period - p.eclipse_duration / 2.0;
    assert_eq!(eclipse_strength(&p, t), 1.0);
    assert_eq!(eclipse_strength(&p, 2.0), 0.0);
    let u = uniforms(t);
    let eclipsed = prepare(&p, &u, DebugView::None);
    let clear = prepare(
        &CrownParams {
            eclipse_depth: 0.0,
            ..p
        },
        &u,
        DebugView::None,
    );
    let eye = u.eye_direction;
    let core = sample(eye);
    assert!(luminance(eclipsed.shade(&core)) < 0.6 * luminance(clear.shade(&core)));
    // A limb direction: almost perpendicular to the eye.
    let (t_axis, _) = rust_software_renderer::shading::sphere::tangent_frame(eye);
    let limb = sample((eye * 0.05 + t_axis).normalized());
    assert!(luminance(eclipsed.shade(&limb)) >= 0.8 * luminance(clear.shade(&limb)));
}

#[test]
fn diadem_is_continuous_across_the_antimeridian() {
    let f = frame(CrownParams::default(), 1.0);
    for lat_deg in [48.0_f32, 52.0, 58.0, 65.0] {
        let lat = lat_deg.to_radians();
        let at = |lon: f32| Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
        let eps = 1e-3;
        let west = f.diadem(at(std::f32::consts::PI - eps));
        let east = f.diadem(at(-std::f32::consts::PI + eps));
        assert!(
            (west - east).abs() < 0.05,
            "lat {lat_deg}: {west} vs {east}"
        );
    }
    // The band itself is present at the base latitude.
    let base = 50.0_f32.to_radians();
    assert!(f.diadem(Vector3::new(base.cos(), base.sin(), 0.0)) > 0.9);
}

// ---- Artistic complement: plasma loops that erupt and drain back ----

use rust_software_renderer::celestial::crown::{LOOP_CONE_DEG, plasma_loop};
use rust_software_renderer::shading::sphere::from_local;

/// Strongest filament core of loop `slot` over directions near its arc.
fn peak_core(f: &Frame, slot: usize) -> f32 {
    let l = f.loops[slot];
    let mut best = 0.0_f32;
    for i in 0..=60 {
        for j in -6..=6 {
            let (x, y) = l.arc_point(i as f32 / 60.0);
            let r = 1.0 + j as f32 * 0.02;
            let n = from_local(l.center, l.east, l.north, x * r, y * r);
            best = best.max(l.glow(n, f.uniforms.time, f.params.loop_width).core);
        }
    }
    best
}

#[test]
fn plasma_loops_are_born_flare_and_drain_back() {
    let p = CrownParams::default();
    let at = |tau: f32| frame(p, tau * p.loop_period);
    // Slot 0 has phase 0, so tau = t / loop_period.
    assert_eq!(
        peak_core(&at(0.05), 0),
        0.0,
        "concentration: no filament yet"
    );
    assert!(
        at(0.05).loops[0].foot > 0.2,
        "the foot point gathers plasma first"
    );
    let birth = peak_core(&at(0.3), 0);
    let maximum = peak_core(&at(0.62), 0);
    let returning = peak_core(&at(0.8), 0);
    assert!(maximum > 0.6, "maximum {maximum}");
    assert!(birth > 0.0 && returning > 0.0);
    assert!(
        at(0.62).loops[0].heat > 0.9,
        "near-white heat at the maximum"
    );
    assert_eq!(peak_core(&at(0.95), 0), 0.0, "stabilized: reabsorbed");
}

#[test]
fn plasma_loops_vanish_at_cycle_boundaries_and_relocate() {
    let p = CrownParams::default();
    for slot in 0..3 {
        for k in 1..4 {
            let period = p.loop_period * (1.0 + 0.37 * slot as f32);
            let boundary = k as f32 * period - 0.45 * slot as f32 * period;
            if boundary <= 0.0 {
                continue;
            }
            for t in [boundary - 1e-3, boundary + 1e-3] {
                let l = plasma_loop(&p, 7, slot, t);
                assert!(
                    l.intensity < 1e-3 && l.foot < 1e-2 && l.heat < 1e-3,
                    "slot {slot} t {t}"
                );
            }
        }
    }
    let (a, b) = (
        plasma_loop(&p, 7, 0, 4.0),
        plasma_loop(&p, 7, 0, 4.0 + p.loop_period),
    );
    assert_ne!(a.cycle, b.cycle);
    assert!(rust_software_renderer::shading::sphere::angular_distance(a.center, b.center) > 0.05);
    let focus = direction_from_angles(-2.45, 0.55);
    for i in 0..60 {
        let l = plasma_loop(&p, 7, i % 3, i as f32 * 1.7);
        let angle = rust_software_renderer::shading::sphere::angular_distance(l.center, focus);
        assert!(angle <= LOOP_CONE_DEG.to_radians() + 1e-4);
    }
}

#[test]
fn loops_switch_restores_the_plain_star() {
    let p = CrownParams::default();
    let off = frame(CrownParams { loops: 0.0, ..p }, 5.6);
    let dim = frame(
        CrownParams {
            loop_brightness: 0.0,
            ..p
        },
        5.6,
    );
    for n in directions(4096) {
        assert_eq!(off.loop_glow(n), Default::default());
        let c = off.shade(&sample(n));
        assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
    }
    // With the loops on they add light somewhere on the visible disk.
    let on = frame(p, 5.6);
    let brighter = directions(4096)
        .into_iter()
        .filter(|&n| luminance(on.shade(&sample(n))) > luminance(dim.shade(&sample(n))) + 0.1)
        .count();
    assert!(brighter > 10, "{brighter}");
}

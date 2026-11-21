//! The Project 3 sky cycles slowly from purple to blue and back: the
//! painted nebula is recolored on its sampling grid with a share of blue
//! that follows the simulated clock and sweeps across the sky, so the
//! whole backdrop never turns one color at once. Every laboratory and
//! pre-project scene keeps the static sky (`drift` 0).

use raylib::math::Vector3;
use rust_software_renderer::benchmark::solar::{SolarBenchConfig, scenario_scene_on};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::nebula::{
    DEEP_PURPLE, MAGENTA, SKY_CYCLE, VOID, WINE, blue_of, blue_share,
};

fn directions(n: usize) -> Vec<Vector3> {
    (0..n)
        .map(|k| {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).sqrt();
            let a = k as f32 * 2.399_963;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

#[test]
fn share_follows_the_cycle() {
    let dirs = directions(500);
    for &d in &dirs {
        // Purple at the start, never out of [0, 1], back to purple after a
        // whole cycle.
        assert_eq!(blue_share(d, 0.0, 1.0), 0.0);
        for k in 0..40 {
            let t = k as f32 * SKY_CYCLE / 40.0;
            let s = blue_share(d, t, 1.0);
            assert!((0.0..=1.0).contains(&s), "{s}");
        }
        assert!(blue_share(d, SKY_CYCLE, 1.0) < 1e-4);
        // Off, or not finite: the painted colors.
        assert_eq!(blue_share(d, SKY_CYCLE / 2.0, 0.0), 0.0);
        assert_eq!(blue_share(d, f32::NAN, 1.0), 0.0);
        assert_eq!(blue_share(d, 10.0, f32::NAN), 0.0);
    }
}

#[test]
fn never_one_color_at_once() {
    // Half a cycle in, the wave leaves some regions much bluer than others.
    let shares: Vec<f32> = directions(500)
        .into_iter()
        .map(|d| blue_share(d, SKY_CYCLE / 2.0, 1.0))
        .collect();
    let (lo, hi) = shares
        .iter()
        .fold((1.0_f32, 0.0_f32), |(l, h), &s| (l.min(s), h.max(s)));
    assert!(lo < 0.25 && hi > 0.9, "{lo} {hi}");
}

#[test]
fn purples_turn_blue() {
    for c in [VOID, DEEP_PURPLE, WINE, MAGENTA] {
        let b = blue_of(c);
        assert!(c.x > c.z * 0.6, "a purple or wine has red");
        assert!(b.z > b.x * 2.5 && b.z >= c.z, "{c:?} -> {b:?}");
    }
}

/// Mean blue minus mean red of the sky of scenario G (facing empty space)
/// at simulated time `t`.
fn sky_tint(t: f32) -> f32 {
    let config = SolarBenchConfig {
        width: 160,
        height: 120,
        time: t,
        ..SolarBenchConfig::default()
    };
    let s = scenario_scene_on("G", &config, true).unwrap();
    assert_eq!(s.nebula.drift, 1.0);
    assert_eq!(s.nebula.time, t);
    let mut fb = Framebuffer::new(160, 120);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    let px = fb.to_rgba_bytes();
    let n = (px.len() / 4) as f32;
    px.chunks(4)
        .map(|p| f32::from(p[2]) - f32::from(p[0]))
        .sum::<f32>()
        / n
}

#[test]
fn rendered_sky_goes_blue_and_back() {
    let start = sky_tint(0.0);
    let middle = sky_tint(SKY_CYCLE / 2.0);
    let end = sky_tint(SKY_CYCLE);
    assert!(middle > start + 4.0, "{start} -> {middle}");
    assert!((end - start).abs() < 1.0, "{start} vs {end}");
}

#[test]
fn historical_scenes_keep_the_static_sky() {
    let config = SolarBenchConfig {
        width: 80,
        height: 60,
        ..SolarBenchConfig::default()
    };
    let legacy = scenario_scene_on("G", &config, false).unwrap();
    assert_eq!(legacy.nebula.drift, 0.0);
}

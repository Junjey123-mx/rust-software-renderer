//! Three monumental waterfalls of the clover planet and its ocean swell
//! (Project 3 expansion):
//! besides the classic fall of every mountain, the first three mountains
//! carry a fall three times as wide that starts higher up, on another
//! flank, and runs down the gradient to the sea; drawn with the same
//! waterfall shader, so 4 + 3 = 7 falls in all. Broad bands of swell
//! cross the open sea on the waves' clock.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarScene;
use rust_software_renderer::solar::attach::waterfalls::{
    MONUMENTAL_APART, MONUMENTAL_FALLS, MONUMENTAL_WIDTH, PlacedWaterfall, WATERFALL_WIDTH,
    WaterfallSet, monumental_from, waterfall_from,
};
use rust_software_renderer::solar::planet::CloverField;
use rust_software_renderer::solar::planet::clover::{CloverParams, WAVE_PERIOD, swell};
use rust_software_renderer::solar::system::PlanetId;

fn field() -> CloverField {
    CloverField::new(7)
}

#[test]
fn three_monumental_falls_reach_the_sea() {
    let f = field();
    let set = WaterfallSet::for_clover(&f, 1, 7, 1.0);
    assert_eq!(set.falls.len(), 4, "the classic falls stay");
    assert_eq!(set.monumental.len(), MONUMENTAL_FALLS);
    assert_eq!(set.falls.len() + set.monumental.len(), 7);
    for (k, m) in set.monumental.iter().enumerate() {
        assert_eq!(m.mountain, k);
        let (top, bottom) = (m.fall.top.normalized(), m.fall.bottom.normalized());
        assert!(f.height(top) > f.sea, "starts on land");
        assert!(
            !f.is_land(bottom) || f.height(bottom) - f.sea < 2e-3,
            "ends at the sea"
        );
        assert!((m.fall.width - MONUMENTAL_WIDTH).abs() < 1e-6);
        assert!(m.fall.width >= 3.0 * WATERFALL_WIDTH - 1e-6);
        // Another flank than the classic fall of its mountain.
        let classic = &set.falls[k];
        let apart = top
            .dot(classic.fall.top.normalized())
            .clamp(-1.0, 1.0)
            .acos();
        assert!(apart > MONUMENTAL_APART, "{apart}");
        assert!(m.soup.iter().all(|v| v.position.x.is_finite()));
        // Over the ground, never under it.
        for v in &m.fall.mesh.vertices {
            let n = v.normalized();
            assert!(v.length() >= 1.0 + f.height(n).max(f.sea) - 1e-3);
        }
    }
    assert!(
        set.reach()
            >= set
                .monumental
                .iter()
                .map(|m| m.fall.top.length())
                .fold(0.0, f32::max)
    );
}

#[test]
fn deterministic_and_off_with_the_classic_falls() {
    let f = field();
    let a = monumental_from(&f, 1, 7).unwrap();
    let b = monumental_from(&f, 1, 7).unwrap();
    assert_eq!(a, b);
    assert_ne!(a.top, waterfall_from(&f, 1, 7).unwrap().top);
    assert!(monumental_from(&f, 99, 7).is_err());
    let none = WaterfallSet::for_clover(&f, 0, 7, 1.0);
    assert!(none.falls.is_empty() && none.monumental.is_empty());
}

/// Lit pixels that differ between the clover close-up with and without the
/// monumental falls.
#[test]
fn drawn_in_the_clover_close_up() {
    let config = SolarBenchConfig {
        width: 320,
        height: 240,
        ..SolarBenchConfig::default()
    };
    let render = |strip: bool| {
        let mut s = scenario_scene_on("K3", &config, true).unwrap();
        aim_at_monumental(&mut s);
        if strip {
            s.waterfalls.as_mut().unwrap().monumental.clear();
        }
        let mut fb = Framebuffer::new(320, 240);
        s.render_frame(
            &mut fb,
            TriangleMode::FilledScanline,
            ShadedOptions::default(),
        )
        .unwrap();
        fb.to_rgba_bytes()
    };
    let (with, without) = (render(false), render(true));
    let changed = with
        .chunks(4)
        .zip(without.chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 50, "{changed} pixels");
}

/// Over the sea below the most sunlit monumental fall, facing it (the
/// `K3` framing of the benchmark, on a monumental fall).
fn aim_at_monumental(s: &mut SolarScene) {
    let clover = s.body_index(PlanetId("clover")).unwrap();
    let t = s.bodies[clover].transform;
    let world = |v: Vector3| t.position + t.to_world_dir(v) * t.scale;
    let sun = (s.center - t.position).normalized();
    let fall = s
        .waterfalls
        .as_ref()
        .unwrap()
        .monumental
        .iter()
        .max_by(|a, b| {
            let lit = |f: &PlacedWaterfall| t.to_world_dir(f.normal).dot(sun);
            lit(a).total_cmp(&lit(b))
        })
        .unwrap()
        .clone();
    let (top, bottom) = (world(fall.fall.top), world(fall.fall.bottom));
    let out = (bottom - t.position).normalized();
    let away = bottom - top;
    let away = (away - out * away.dot(out)).normalized();
    let r = s.radius_of(clover);
    s.ship.position = bottom + away * (0.3 * r) + out * (0.12 * r);
    s.ship.forward = ((top + bottom) * 0.5 - s.ship.position).normalized();
    s.ship.up = out;
    s.ship.renormalize();
}

#[test]
fn swell_travels_and_stops_with_the_waves() {
    let p = CloverParams::default();
    let dirs: Vec<Vector3> = (0..400)
        .map(|k| {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / 400.0;
            let r = (1.0 - y * y).sqrt();
            let a = k as f32 * 2.399_963;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect();
    let at = |t: f32, p: &CloverParams| dirs.iter().map(|&n| swell(n, t, p)).collect::<Vec<_>>();
    let (a, b) = (at(0.0, &p), at(3.0, &p));
    assert!(a.iter().chain(&b).all(|s| (0.0..=1.0).contains(s)));
    // Bands, not a uniform glow, and they move.
    assert!(a.iter().filter(|&&s| s > 0.5).count() > 20);
    assert!(a.iter().filter(|&&s| s == 0.0).count() > 100);
    assert!(
        a.iter()
            .zip(&b)
            .filter(|(x, y)| (*x - *y).abs() > 0.1)
            .count()
            > 40
    );
    // Periodic, and frozen with the waves.
    let c = at(WAVE_PERIOD, &p);
    assert!(a.iter().zip(&c).all(|(x, y)| (x - y).abs() < 1e-3));
    let still = CloverParams {
        wave_speed: 0.0,
        ..p
    };
    assert_eq!(at(0.0, &still), at(17.0, &still));
    assert!(swell(dirs[0], f32::NAN, &p).is_finite());
}

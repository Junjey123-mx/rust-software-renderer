//! Clover Haven, second pass: a luminous, moving sea and waterfalls that
//! read from afar. The monumental falls leave their lip and drop as a
//! curtain over the slope into the sea; every fall is double-sided and
//! drawn from farther away than the props; the lit sea is brighter than
//! the first pass's navy and its currents drift with time.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::props::PROP_MIN_PX;
use rust_software_renderer::solar::attach::waterfalls::{
    FALL_MIN_PX, MONUMENTAL_WIDTH, WATERFALL_GAP, WaterfallSet,
};
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::CloverField;
use rust_software_renderer::solar::planet::clover::{CloverParams, PreparedClover, current};
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

/// Height of `p` over the ground (or the sea) right below it.
fn over_surface(field: &CloverField, p: Vector3) -> f32 {
    let n = p.normalized();
    p.length() - 1.0 - field.height(n).max(field.sea)
}

#[test]
fn monumental_curtains_rise_over_the_slope_and_end_at_the_sea() {
    for seed in [7, 99, 2024] {
        let f = CloverField::new(seed);
        let set = WaterfallSet::for_clover(&f, 1, seed, 1.0);
        assert_eq!(set.monumental.len(), 3, "seed {seed}");
        for m in &set.monumental {
            let v = &m.fall.mesh.vertices;
            // Rung centers of the curtain (the spray dome follows them).
            let rungs: Vec<Vector3> = (0..=24).map(|i| (v[2 * i] + v[2 * i + 1]) * 0.5).collect();
            let rise: Vec<f32> = rungs.iter().map(|&p| over_surface(&f, p)).collect();
            // Off the ground along its drop: clearly in the air mid-way.
            let highest = rise.iter().cloned().fold(0.0, f32::max);
            assert!(highest > 0.015, "seed {seed}: {rise:?}");
            assert!(
                rise[8..17].iter().all(|&h| h > WATERFALL_GAP + 1e-3),
                "{rise:?}"
            );
            // Never under the ground, and it lands on the sea.
            assert!(rise.iter().all(|&h| h > 0.0));
            let foot = rungs[24].normalized();
            assert!(!f.is_land(foot) || f.height(foot) - f.sea < 2e-3);
            // Its last rung lies on the water (level within a little tilt).
            assert!(rise[24] < 0.012, "{}", rise[24]);
            // It falls a long way: the lip stands well over the sea.
            assert!(f.height(rungs[0].normalized()) - f.sea > 0.02);
            // Much wider than the first pass (0.06): it spreads as it
            // falls and narrows only where a bay's walls close in.
            const { assert!(MONUMENTAL_WIDTH >= 0.12) };
            let widths: Vec<f32> = (0..=24)
                .map(|i| (v[2 * i + 1] - v[2 * i]).length())
                .collect();
            let mean = widths.iter().sum::<f32>() / widths.len() as f32;
            assert!(mean > 0.1, "{widths:?}");
            assert!(widths.iter().all(|&w| w > 2.0 * 0.02 - 1e-3), "{widths:?}");
            // A spray dome over the water at the foot.
            assert!(v.len() > 2 * 25, "spray vertices");
            let apex = v[2 * 25];
            assert!(over_surface(&f, apex) > 0.02, "{}", over_surface(&f, apex));
        }
    }
}

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(400, 300, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap()
}

/// The ship `d` radii from the clover's center, looking at it.
fn looking_at_clover(d: f32) -> SolarScene {
    let mut s = scene();
    let i = s.body_index(PlanetId("clover")).unwrap();
    let p = s.bodies[i].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(i));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

#[test]
fn falls_are_double_sided() {
    let s = looking_at_clover(2.5);
    let draws = s.effect_draws(400, 300, 1);
    let falls: Vec<_> = draws
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Waterfall(_)))
        .collect();
    assert!(!falls.is_empty());
    for d in &falls {
        assert!(!d.options.cull_back_faces, "both faces drawn");
        let EffectShader::Waterfall(w) = d.shader else {
            unreachable!()
        };
        // Both faces lit alike.
        let p = (w.top + w.bottom) * 0.5;
        let n = (w.bottom - w.top).cross(w.top).normalized();
        assert_eq!(w.shade(p, n), w.shade(p, -n));
    }
}

#[test]
fn falls_drawn_from_farther_than_the_props() {
    const { assert!(FALL_MIN_PX < 0.5 * PROP_MIN_PX) };
    let count = |s: &SolarScene| {
        let draws = s.effect_draws(400, 300, 1);
        let falls = draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Waterfall(_)))
            .count();
        let props = draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Prop(_)))
            .count();
        (falls, props)
    };
    // Far enough that the props are hidden, the falls still show.
    let far = (6..40)
        .map(|d| d as f32)
        .find(|&d| count(&looking_at_clover(d)).1 == 0)
        .expect("the props hide with distance");
    let (falls, props) = count(&looking_at_clover(far));
    assert_eq!(props, 0);
    assert!(falls > 0, "at {far} radii");
    // Twice as far still shows them.
    assert!(count(&looking_at_clover(2.0 * far)).0 > 0);
}

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

fn prepared(time: f32) -> PreparedClover {
    let params = CloverParams::default();
    PreparedClover {
        field: params.field(),
        params,
        light: Vector3::new(-0.4, 0.7, 0.6).normalized(),
        time,
    }
}

/// Sea directions spread over the sphere.
fn sea(p: &PreparedClover) -> Vec<Vector3> {
    (0..600)
        .map(|k| {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / 600.0;
            let r = (1.0 - y * y).sqrt();
            let a = k as f32 * 2.399_963;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .filter(|&n| !p.field.is_land(n))
        .collect()
}

#[test]
fn lit_sea_is_luminous_blue() {
    let p = prepared(0.0);
    // Lit at 30-60 degrees and seen from straight above: no star glint.
    let lit: Vec<Vector3> = sea(&p)
        .into_iter()
        .filter(|&n| (0.5..0.87).contains(&n.dot(p.light)))
        .collect();
    assert!(lit.len() > 50);
    let mean = lit
        .iter()
        .map(|&n| p.shade(n, n))
        .fold(Vector3::zero(), |a, c| a + c)
        / lit.len() as f32;
    let diffuse = lit.iter().map(|&n| n.dot(p.light)).sum::<f32>() / lit.len() as f32;
    // The first pass's deep navy #0A2F66 (luminance 0.17) under the same
    // light, with its swell: about 0.13 here. Now well over twice that.
    let before = 0.17 * diffuse + 0.03;
    assert!(luminance(mean) > 2.0 * before, "{mean:?} vs {before}");
    assert!(
        mean.z > mean.x && mean.y > 2.0 * mean.x,
        "turquoise-blue: {mean:?}"
    );
}

#[test]
fn sea_currents_drift() {
    let p = prepared(0.0);
    let dirs = sea(&p);
    let params = CloverParams::default();
    let at = |t: f32| {
        dirs.iter()
            .map(|&n| current(n, t, &params))
            .collect::<Vec<_>>()
    };
    let (a, b) = (at(0.0), at(2.0));
    let moved = a
        .iter()
        .zip(&b)
        .filter(|(x, y)| (*x - *y).abs() > 0.05)
        .count();
    assert!(moved * 4 > dirs.len(), "{moved} of {}", dirs.len());
    // Streaks, not a flat tint: some open sea is lit by them (thin ones,
    // with calm patches between), most is not.
    let streaky = a.iter().filter(|&&c| c > 0.5).count();
    assert!(
        streaky > dirs.len() / 40 && streaky < dirs.len() / 2,
        "{streaky}"
    );
    // Repeats with the waves and stops with them.
    let wrapped = at(2.0 + rust_software_renderer::solar::planet::clover::WAVE_PERIOD);
    assert!(b.iter().zip(&wrapped).all(|(x, y)| (x - y).abs() < 1e-3));
    let still = CloverParams {
        wave_speed: 0.0,
        ..params
    };
    assert!(
        dirs.iter()
            .all(|&n| current(n, 0.0, &still) == current(n, 7.0, &still))
    );
    // The shaded sea changes between frames too.
    let shade = |t: f32| {
        let q = prepared(t);
        dirs.iter().map(|&n| q.shade(n, n)).collect::<Vec<_>>()
    };
    let (x, y) = (shade(0.0), shade(2.0));
    let changed = x
        .iter()
        .zip(&y)
        .filter(|(c, d)| (**c - **d).length() > 0.02)
        .count();
    assert!(changed * 4 > dirs.len(), "{changed}");
}

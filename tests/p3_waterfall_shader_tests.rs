//! Project 3 waterfall water: bands that slide down the ribbon with the
//! scene clock, freeze while paused (and with no speed), and foam at the
//! base.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::waterfalls::{
    Waterfall, WaterfallShader, waterfall_from,
};
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::planet::CloverField;
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn fall() -> Waterfall {
    waterfall_from(&CloverField::new(7), 0, 7).unwrap()
}

/// The ribbon's outward normal, lit from straight above.
fn shader(w: &Waterfall, time: f32) -> (WaterfallShader, Vector3) {
    let up = w.top.normalized();
    (w.shader(up, time), up)
}

/// Colors at the middle of every rung.
fn colors(w: &Waterfall, s: &WaterfallShader, normal: Vector3) -> Vec<Vector3> {
    w.mesh
        .vertices
        .chunks(2)
        .map(|r| s.shade((r[0] + r[1]) * 0.5, normal))
        .collect()
}

#[test]
fn paused_freezes_bands() {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut scene = SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    scene.clock.time = 3.0;
    apply_toggles(&mut scene, &[SolarAction::TogglePause]);
    let w = fall();
    let (before, n) = shader(&w, scene.clock.time);
    for _ in 0..30 {
        step(&mut scene, &[], &[], Look::default(), 1.0 / 30.0);
    }
    let (after, _) = shader(&w, scene.clock.time);
    assert_eq!(colors(&w, &before, n), colors(&w, &after, n));
}

#[test]
fn time_moves_bands() {
    let w = fall();
    let (a, n) = shader(&w, 1.0);
    let (b, _) = shader(&w, 1.3);
    let changed = colors(&w, &a, n)
        .iter()
        .zip(colors(&w, &b, n))
        .filter(|(x, y)| (**x - *y).length() > 0.02)
        .count();
    assert!(changed >= 4, "{changed} rungs changed");
    // One whole band later the pattern is back.
    let (c, _) = shader(&w, 1.0 + 1.0 / a.speed);
    for (x, y) in colors(&w, &a, n).iter().zip(colors(&w, &c, n)) {
        assert!((*x - y).length() < 1e-3);
    }
}

#[test]
fn foam_at_base() {
    let w = fall();
    let (s, n) = shader(&w, 2.0);
    let c = colors(&w, &s, n);
    let brightness = |v: Vector3| v.x + v.y + v.z;
    let base = brightness(c[c.len() - 1]);
    let top: f32 = c[..4].iter().map(|&v| brightness(v)).sum::<f32>() / 4.0;
    assert!(base > top + 0.3, "base {base}, top {top}");
    assert!(s.along(w.top) < 1e-6 && (s.along(w.bottom) - 1.0).abs() < 1e-6);
    assert!(
        c.iter()
            .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
    );
}

#[test]
fn no_speed_is_static() {
    let w = fall();
    let (mut a, n) = shader(&w, 0.0);
    a.speed = 0.0;
    let mut b = a;
    b.time = 1234.5;
    assert_eq!(colors(&w, &a, n), colors(&w, &b, n));
}

//! The Joker, fifth planet of Project 3: its own faceted relief and
//! harlequin shader (not a recolor of another planet), static geometry
//! under animated colors, and its place in the system (the fifth slot).

use raylib::math::Vector3;
use rust_software_renderer::solar::planet::HeightField;
use rust_software_renderer::solar::planet::field::NORMAL_EPS;
use rust_software_renderer::solar::planet::hearts::NeonGridParams;
use rust_software_renderer::solar::planet::joker::{
    JOKER_HEIGHT, JokerField, JokerParams, PreparedJoker, diamond_glitch,
};
use rust_software_renderer::solar::planet::{CloverField, HeartsField};
use rust_software_renderer::solar::system::spec::{FIFTH_PLANET, FifthPlanetStatus};
use rust_software_renderer::solar::system::{GeometrySpec, ScaleProfile, SystemSpec};

/// Fixed directions spread over the sphere (a golden spiral).
fn samples(n: usize) -> Vec<Vector3> {
    (0..n)
        .map(|k| {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).sqrt();
            let a = k as f32 * 2.399_963;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect()
}

fn joker() -> HeightField {
    HeightField::Joker(JokerField::new(23))
}

fn prepared(time: f32) -> PreparedJoker {
    let params = JokerParams::default();
    PreparedJoker {
        field: params.field(),
        params,
        light: Vector3::new(0.4, 0.6, 0.7).normalized(),
        time,
    }
}

#[test]
fn field_finite_and_bounded() {
    let f = joker();
    let mut top: f32 = 0.0;
    for n in samples(4000) {
        let h = f.height(n);
        assert!(
            h.is_finite() && (0.0..=JOKER_HEIGHT + 1e-6).contains(&h),
            "{h}"
        );
        top = top.max(h);
    }
    assert!(top > 0.6 * JOKER_HEIGHT, "tallest diamond {top}");
    assert_eq!(f.max_height(), JOKER_HEIGHT);
    assert!(f.validate().is_ok());
    // Negative cases: a hand-built field with bad values is refused.
    let bad = JokerField {
        cells: 1.0,
        ..JokerField::new(1)
    };
    assert!(HeightField::Joker(bad).validate().is_err());
    let nan = JokerField {
        height: f32::NAN,
        ..JokerField::new(1)
    };
    assert!(HeightField::Joker(nan).validate().is_err());
}

#[test]
fn normals_unit_and_outward() {
    let f = joker();
    for n in samples(1500) {
        let m = f.normal(n, NORMAL_EPS);
        assert!((m.length() - 1.0).abs() < 1e-4, "{m:?}");
        assert!(m.dot(n) > 0.0);
    }
}

#[test]
fn relief_is_faceted_and_asymmetric() {
    let f = joker();
    // Facets: many samples tilt away from the radial normal.
    let tilted = samples(2000)
        .into_iter()
        .filter(|&n| f.normal(n, NORMAL_EPS).dot(n) < 0.995)
        .count();
    assert!(tilted > 400, "{tilted}");
    // One hemisphere (+X) stands taller than the other.
    let mean = |side: f32| {
        let v: Vec<f32> = samples(4000)
            .into_iter()
            .filter(|n| n.x * side > 0.3)
            .map(|n| f.height(n))
            .collect();
        v.iter().sum::<f32>() / v.len() as f32
    };
    assert!(mean(1.0) > 1.3 * mean(-1.0), "{} {}", mean(1.0), mean(-1.0));
}

#[test]
fn seed_changes_the_relief_and_is_reproducible() {
    let a = HeightField::Joker(JokerField::new(23));
    let b = HeightField::Joker(JokerField::new(24));
    let again = HeightField::Joker(JokerField::new(23));
    let s = samples(800);
    assert!(s.iter().all(|&n| a.height(n) == again.height(n)));
    assert!(s.iter().filter(|&&n| a.height(n) != b.height(n)).count() > 100);
}

/// Quantized colors of `shade` over fixed samples, for comparing shaders.
fn digest(colors: impl Iterator<Item = Vector3>) -> Vec<[u8; 3]> {
    colors
        .map(|c| {
            let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
            [q(c.x), q(c.y), q(c.z)]
        })
        .collect()
}

#[test]
fn distinct_from_the_other_planets() {
    let s = samples(600);
    let j = digest(s.iter().map(|&n| prepared(1.0).shade(n, n)));
    // The Hearts grid on the same samples.
    let params = NeonGridParams::default();
    let hearts = rust_software_renderer::solar::planet::hearts::PreparedHearts {
        field: params.field(),
        params,
        light: Vector3::new(0.4, 0.6, 0.7).normalized(),
        time: 1.0,
    };
    let h = digest(s.iter().map(|&n| hearts.shade(n)));
    let same = j.iter().zip(&h).filter(|(a, b)| a == b).count();
    assert!(same < s.len() / 20, "{same} equal colors");
    // Its own palette: emerald (green over red) and purple (blue over
    // green) diamonds both appear.
    let c = |v: &[u8; 3]| v.map(i32::from);
    let green = j
        .iter()
        .map(c)
        .filter(|c| c[1] > c[0] + 20 && c[1] > c[2])
        .count();
    let purple = j
        .iter()
        .map(c)
        .filter(|c| c[2] > c[1] + 15 && c[0] > c[1])
        .count();
    assert!(green > 30 && purple > 30, "{green} {purple}");
    // And its relief differs from the clover's and the Hearts' fields.
    let others = [
        HeightField::Clover(CloverField::new(7)),
        HeightField::Hearts(HeartsField::new(11)),
    ];
    for other in others {
        let differ = s
            .iter()
            .filter(|&&n| (joker().height(n) - other.height(n)).abs() > 1e-3)
            .count();
        assert!(differ > s.len() / 2, "{differ}");
    }
}

#[test]
fn colors_animate_but_the_relief_does_not() {
    let s = samples(600);
    let a = digest(s.iter().map(|&n| prepared(0.0).shade(n, n)));
    let b = digest(s.iter().map(|&n| prepared(2.5).shade(n, n)));
    let changed = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    assert!(changed > 60, "{changed}");
    // Same time, same colors (deterministic, frozen while paused).
    assert_eq!(a, digest(s.iter().map(|&n| prepared(0.0).shade(n, n))));
    // The relief has no time input: the field is the same object.
    assert_eq!(prepared(0.0).field, prepared(2.5).field);
}

#[test]
fn glitch_is_deterministic_and_switchable() {
    let p = JokerParams::default();
    let cells: Vec<(i32, i32)> = (0..30).flat_map(|i| (0..30).map(move |j| (i, j))).collect();
    let count =
        |t: f32, p: &JokerParams| cells.iter().filter(|&&c| diamond_glitch(c, t, p)).count();
    let n = count(0.4, &p);
    assert!(n > 0 && n < cells.len() / 4, "{n}");
    assert_eq!(n, count(0.4, &p));
    let off = JokerParams { glitch: 0.0, ..p };
    assert_eq!(count(0.4, &off), 0);
    assert_eq!(count(f32::NAN, &p), 0);
}

#[test]
fn fifth_planet_of_the_system() {
    assert_eq!(FIFTH_PLANET, FifthPlanetStatus::Approved("Joker"));
    let spec = SystemSpec::project3();
    assert_eq!(spec.planets.len(), 5);
    let j = &spec.planets[4];
    assert_eq!((j.id.0, j.geometry), ("joker", GeometrySpec::Joker));
    // Beyond House of Cards, slower than it, within the scale band.
    assert!(j.orbit_radius > spec.planets[3].orbit_radius);
    assert!(j.orbit_speed > 0.0 && j.orbit_speed < spec.planets[3].orbit_speed);
    let ratio = j.radius() / ScaleProfile::project3().ship_radius;
    assert!((3.0..=6.0).contains(&ratio), "{ratio}");
    assert!(j.mu > 0.0);
}

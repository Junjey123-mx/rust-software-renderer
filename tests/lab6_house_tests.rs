//! House of Cards: liquid latitude bands with differential winds.

use raylib::math::Vector3;
use rust_software_renderer::celestial::DebugView;
use rust_software_renderer::celestial::house::{Frame, HouseParams, prepare};
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

fn frame_at(params: HouseParams, time: f32) -> Frame {
    let u = ShaderUniforms {
        time,
        light_direction: direction_from_angles(-1.3, 0.35),
        eye_direction: direction_from_angles(-2.45, 0.55),
    };
    prepare(&params, &u, DebugView::None)
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

fn lat_lon(lat: f32, lon: f32) -> Vector3 {
    Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
}

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

#[test]
fn bands_vary_mainly_with_latitude() {
    let f = frame_at(HouseParams::default(), 0.0);
    let rows: Vec<Vec<f32>> = (0..64)
        .map(|i| {
            let lat = -1.3 + 2.6 * i as f32 / 63.0;
            (0..64)
                .map(|j| {
                    let lon = -std::f32::consts::PI + std::f32::consts::TAU * j as f32 / 64.0;
                    luminance(f.albedo(lat_lon(lat, lon)))
                })
                .collect()
        })
        .collect();
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let var = |v: &[f32]| {
        let m = mean(v);
        v.iter().map(|x| (x - m).powi(2)).sum::<f32>() / v.len() as f32
    };
    let row_means: Vec<f32> = rows.iter().map(|r| mean(r)).collect();
    let across_latitude = var(&row_means);
    let along_longitude = mean(&rows.iter().map(|r| var(r)).collect::<Vec<_>>());
    assert!(
        across_latitude > 4.0 * along_longitude,
        "{across_latitude} vs {along_longitude}"
    );
}

#[test]
fn band_boundaries_ripple_only_with_warp() {
    // Latitude at which the band coordinate crosses a fixed level, per longitude.
    let crossing_spread = |params: HouseParams| {
        let f = frame_at(params, 0.0);
        let level = 0.25;
        let crossings: Vec<f32> = (0..48)
            .map(|j| {
                let lon = -3.0 + 6.0 * j as f32 / 47.0;
                let (mut lo, mut hi) = (-0.2_f32, 0.8_f32);
                for _ in 0..40 {
                    let mid = 0.5 * (lo + hi);
                    if f.band_coordinate(lat_lon(mid, lon)) < level {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                0.5 * (lo + hi)
            })
            .collect();
        let m = crossings.iter().sum::<f32>() / crossings.len() as f32;
        (crossings.iter().map(|c| (c - m).powi(2)).sum::<f32>() / crossings.len() as f32).sqrt()
    };
    let rippled = crossing_spread(HouseParams::default());
    let straight = crossing_spread(HouseParams {
        band_warp: 0.0,
        ..HouseParams::default()
    });
    assert!(rippled > 0.01, "{rippled}");
    assert!(straight < 1e-4, "{straight}");
}

#[test]
fn bands_flow_over_time() {
    let (a, b) = (
        frame_at(HouseParams::default(), 0.0),
        frame_at(HouseParams::default(), 5.0),
    );
    let dirs = directions(2048);
    let changed = dirs
        .iter()
        .filter(|&&n| (a.albedo(n) - b.albedo(n)).length() > 0.02)
        .count();
    assert!(changed > dirs.len() / 4, "{changed}");
}

#[test]
fn bands_are_continuous_across_the_antimeridian_and_poles() {
    let f = frame_at(HouseParams::default(), 3.0);
    let eps = 1e-3;
    for i in 0..20 {
        let lat = -1.2 + 2.4 * i as f32 / 19.0;
        let west = f.albedo(lat_lon(lat, std::f32::consts::PI - eps));
        let east = f.albedo(lat_lon(lat, -std::f32::consts::PI + eps));
        assert!((west - east).length() < 0.02, "lat {lat}");
    }
    for pole in [1.0_f32, -1.0] {
        let p = Vector3::new(0.0, pole, 0.0);
        for k in 0..8 {
            let a = k as f32 * 0.8;
            let near = Vector3::new(1e-3 * a.cos(), pole, 1e-3 * a.sin()).normalized();
            let c = f.shade(&sample(near));
            assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
            assert!((f.albedo(near) - f.albedo(p)).length() < 0.02);
        }
    }
}

#[test]
fn palette_is_predominantly_violet() {
    let f = frame_at(HouseParams::default(), 0.0);
    let dirs = directions(4096);
    let (g, b) = dirs.iter().fold((0.0, 0.0), |(g, b), &n| {
        let c = f.albedo(n);
        (g + c.y, b + c.z)
    });
    let count = dirs.len() as f32;
    assert!(
        b / count > g / count + 0.15,
        "g {} b {}",
        g / count,
        b / count
    );
}

// ---- C15: migrating diamond vortex (E05) ----

use rust_software_renderer::celestial::house::{contour_param, contour_point};
use rust_software_renderer::shading::sphere::{angular_distance, from_local};

/// Apogee of the principal storm (tau = 0.55 of an 18 s cycle).
const APOGEE: f32 = 9.9;

fn no_secondary() -> HouseParams {
    HouseParams {
        secondary_strength: 0.0,
        ..HouseParams::default()
    }
}

#[test]
fn vortex_is_strictly_local() {
    let on = frame_at(no_secondary(), APOGEE);
    let off = frame_at(
        HouseParams {
            vortex: 0.0,
            ..no_secondary()
        },
        APOGEE,
    );
    let storm = on.storms[0];
    let (mut inside, mut changed) = (0, 0);
    for n in directions(16_384) {
        match storm.coords(n) {
            None => assert_eq!(on.shade(&sample(n)), off.shade(&sample(n))),
            Some((d, _)) if d <= 0.6 => {
                inside += 1;
                if (on.shade(&sample(n)) - off.shade(&sample(n))).length() > 0.02 {
                    changed += 1;
                }
            }
            _ => {}
        }
    }
    assert!(inside > 50, "{inside}");
    assert!(changed * 10 >= inside * 3, "{changed} of {inside}");
}

#[test]
fn vortex_displaces_sampling_inside_only() {
    let f = frame_at(no_secondary(), APOGEE);
    let storm = f.storms[0];
    let mut displaced = 0;
    let mut inside = 0;
    for n in directions(8192) {
        let moved = angular_distance(n, f.vortex_warp(n));
        match storm.coords(n) {
            None => assert_eq!(f.vortex_warp(n), n),
            Some((d, _)) if (0.05..0.6).contains(&d) => {
                inside += 1;
                if moved > 1.0_f32.to_radians() {
                    displaced += 1;
                }
            }
            _ => {}
        }
    }
    assert!(
        inside > 0 && displaced * 10 >= inside * 9,
        "{displaced} of {inside}"
    );
}

#[test]
fn diamond_contour_round_trips() {
    let (a, b) = (0.45, 0.6);
    for i in 0..1000 {
        let angle = i as f32 * 0.0137 * std::f32::consts::TAU;
        let r = 0.05 + 0.9 * ((i * 37) % 100) as f32 / 100.0;
        let (u, v) = (r * a * angle.cos(), r * b * angle.sin());
        let d = u.abs() / a + v.abs() / b;
        let s = contour_param(u, v, a, b, d);
        assert!((0.0..4.0).contains(&s), "{s}");
        let (u2, v2) = contour_point(s, d, a, b);
        assert!(
            (u - u2).abs() < 1e-4 && (v - v2).abs() < 1e-4,
            "({u}, {v}) -> ({u2}, {v2})"
        );
    }
}

#[test]
fn vortex_migrates_between_cycles() {
    let period = HouseParams::default().vortex_period;
    let centers: Vec<Vector3> = (0..8)
        .map(|k| frame_at(HouseParams::default(), APOGEE + k as f32 * period).storms[0].center)
        .collect();
    for pair in centers.windows(2) {
        assert!(angular_distance(pair[0], pair[1]) > 0.05);
    }
    let f = frame_at(HouseParams::default(), APOGEE);
    let g = frame_at(HouseParams::default(), APOGEE + period);
    assert_ne!(f.storms[0].cycle, g.storms[0].cycle);
}

#[test]
fn vortex_strength_is_zero_at_cycle_boundaries() {
    let period = HouseParams::default().vortex_period;
    for k in 1..4 {
        for t in [k as f32 * period - 1e-3, k as f32 * period + 1e-3] {
            let f = frame_at(HouseParams::default(), t);
            assert!(f.storms[0].w.abs() < 1e-3, "t {t}: {}", f.storms[0].w);
        }
    }
    let peak = frame_at(HouseParams::default(), APOGEE).storms[0].w;
    assert!((peak - HouseParams::default().vortex_strength).abs() < 1e-3);
}

#[test]
fn vortex_centers_respect_the_latitude_limit() {
    for max in [10.0_f32, 30.0, 45.0] {
        let params = HouseParams {
            vortex_max_latitude: max,
            ..HouseParams::default()
        };
        for i in 0..100 {
            let f = frame_at(params, i as f32 * 4.1);
            for storm in &f.storms {
                let lat = storm.center.y.asin().to_degrees();
                assert!(lat.abs() <= max + 1e-3, "{lat} > {max}");
                assert!((storm.center.length() - 1.0).abs() < 1e-4);
            }
        }
    }
}

#[test]
fn vortex_switch_restores_plain_bands() {
    let off = frame_at(
        HouseParams {
            vortex: 0.0,
            ..HouseParams::default()
        },
        APOGEE,
    );
    for n in directions(4096) {
        assert_eq!(off.vortex_warp(n), n);
        assert_eq!(off.albedo(n), off.band_albedo(n));
    }
    // The storm interior is still visible as a plane region of the diamond.
    let storm = frame_at(HouseParams::default(), APOGEE).storms[0];
    let inner = from_local(storm.center, storm.east, storm.north, 0.1, 0.0);
    assert!(storm.coords(inner).is_some());
}

// ---- Artistic complement: neon lightning in the storms ----

use rust_software_renderer::celestial::house::{STRIKE_SLOTS, strike};

#[test]
fn lightning_is_off_when_disabled() {
    let off = HouseParams {
        lightning: 0.0,
        ..HouseParams::default()
    };
    for i in 0..40 {
        let f = frame_at(off, 8.0 + i as f32 * 0.07);
        assert!(f.strikes.iter().all(|s| s.flash == 0.0));
        for n in directions(256) {
            assert_eq!(f.lightning(n), Vector3::zero());
        }
    }
}

#[test]
fn lightning_flashes_are_brief_and_intermittent() {
    let p = HouseParams::default();
    let storm = frame_at(p, 0.0).storms[0];
    for (c, &slot) in STRIKE_SLOTS.iter().enumerate().skip(1) {
        let slots = 400;
        let active = (0..slots)
            .filter(|&k| strike(&p, &storm, c, (k as f32 + 0.02) * slot).flash > 0.0)
            .count();
        assert!(
            (10..=200).contains(&active),
            "channel {c}: {active} of {slots}"
        );
        // Within a striking slot the flash fades quickly.
        let k = (0..slots)
            .find(|&k| strike(&p, &storm, c, (k as f32 + 0.02) * slot).flash > 0.0)
            .unwrap() as f32;
        let early = strike(&p, &storm, c, (k + 0.02) * slot).flash;
        let late = strike(&p, &storm, c, (k + 0.95) * slot).flash;
        assert!(late < 0.35 * early, "{early} -> {late}");
    }
}

#[test]
fn storm_lightning_stays_inside_the_diamond_and_follows_its_life() {
    let p = HouseParams::default();
    let mut apogee_strikes = 0;
    for i in 0..200 {
        // Around the apogee (tau 0.45-0.65 of the 18 s cycle).
        let f = frame_at(p, 8.1 + i as f32 * 0.018);
        let st = f.strikes[0];
        if st.flash > 0.0 {
            apogee_strikes += 1;
            assert!(
                f.storms[0].coords(st.center).is_some(),
                "bolt seeded in the diamond"
            );
        }
        for n in directions(512) {
            if f.storms[0].coords(n).is_none()
                && f.strikes[1].flash == 0.0
                && f.strikes[2].flash == 0.0
            {
                assert_eq!(
                    f.lightning(n),
                    Vector3::zero(),
                    "storm bolts never leave the diamond"
                );
            }
        }
    }
    assert!(apogee_strikes > 20, "{apogee_strikes}");
    // During the calm phase the storm channel never strikes.
    for i in 0..100 {
        let f = frame_at(p, 0.05 + i as f32 * 0.017);
        assert_eq!(f.strikes[0].flash, 0.0);
    }
}

#[test]
fn lightning_is_neon_violet_and_pink_without_saturating() {
    let p = HouseParams::default();
    let eye = direction_from_angles(-2.45, 0.55);
    let visible: Vec<Vector3> = directions(8192)
        .into_iter()
        .filter(|n| n.dot(eye) > 0.0)
        .collect();
    let mut bolts = 0;
    for i in 0..60 {
        let f = frame_at(p, 9.0 + i as f32 * 0.033);
        let lit: Vec<Vector3> = visible.iter().map(|&n| f.lightning(n)).collect();
        let strong = lit.iter().filter(|c| c.length() > 0.3).count();
        assert!(
            strong * 8 < visible.len(),
            "lightning covers too much of the disk ({strong})"
        );
        for c in lit.iter().filter(|c| c.length() > 0.3) {
            bolts += 1;
            assert!(c.z > c.y && c.x > c.y, "neon violet or pink: {c:?}");
        }
    }
    assert!(bolts > 0);
}

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

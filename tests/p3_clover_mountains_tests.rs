//! Project 3 clover relief: four clover mountains, at least 70 degrees
//! apart, each with its own height, nothing on the far side of the planet,
//! and centers that move with the seed.

use raylib::math::Vector3;
use rust_software_renderer::solar::planet::HeightField;
use rust_software_renderer::solar::planet::clover::{
    CloverField, MIN_CENTER_ANGLE, clover_sdf, tangent_uv,
};
use rust_software_renderer::solar::planet::geometry::GeometryKey;
use rust_software_renderer::solar::planet::icosphere::icosphere;

fn angle(a: Vector3, b: Vector3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

/// Directions in a cap of `radius` radians around `center`.
fn cap(center: Vector3, radius: f32) -> Vec<Vector3> {
    let t1 = center.cross(Vector3::new(0.3, 1.0, 0.2)).normalized();
    let t2 = center.cross(t1);
    let mut out = Vec::new();
    for a in 0..40 {
        for b in 0..40 {
            let u = (a as f32 / 39.0 * 2.0 - 1.0) * radius;
            let v = (b as f32 / 39.0 * 2.0 - 1.0) * radius;
            out.push((center + t1 * u + t2 * v).normalized());
        }
    }
    out
}

#[test]
fn four_peaks_found() {
    for seed in [0, 7, 1234] {
        let f = CloverField::new(seed);
        for (i, &c) in f.centers.iter().enumerate() {
            let (peak, at) = cap(c, 0.45)
                .into_iter()
                .map(|n| (f.height(n), n))
                .fold((f32::MIN, c), |a, b| if b.0 > a.0 { b } else { a });
            assert!(peak > f.sea + 0.05, "seed {seed} clover {i}: {peak}");
            assert!(peak > 0.7 * f.heights[i], "seed {seed} clover {i}");
            // The peak belongs to this clover.
            let nearest = (0..4)
                .min_by(|&a, &b| angle(at, f.centers[a]).total_cmp(&angle(at, f.centers[b])))
                .unwrap();
            assert_eq!(nearest, i);
        }
    }
}

#[test]
fn centers_at_least_70_degrees_apart() {
    for seed in 0..300 {
        let f = CloverField::new(seed);
        f.validate().unwrap();
        for i in 0..4 {
            assert!((f.centers[i].length() - 1.0).abs() < 1e-5);
            for j in i + 1..4 {
                assert!(angle(f.centers[i], f.centers[j]) >= MIN_CENTER_ANGLE);
            }
        }
    }
    // A hand-built field with two mountains 60 degrees apart is refused.
    let mut f = CloverField::new(3);
    let axis = f.centers[0].cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
    let (s, c) = 60f32.to_radians().sin_cos();
    let c0 = f.centers[0];
    f.centers[1] = (c0 * c + axis.cross(c0) * s).normalized();
    assert!(f.validate().is_err());
    let mut f = CloverField::new(3);
    f.lobe_r = f32::NAN;
    assert!(f.validate().is_err());
}

#[test]
fn independent_heights() {
    let f = CloverField::new(42);
    let mut h = f.heights.to_vec();
    h.sort_by(f32::total_cmp);
    h.dedup();
    assert_eq!(h.len(), 4, "{:?}", f.heights);
    // Raising one mountain leaves the others as they were.
    let mut taller = f;
    taller.heights[0] *= 1.5;
    for i in 1..4 {
        for n in cap(f.centers[i], 0.4) {
            assert_eq!(f.height(n), taller.height(n));
        }
    }
    assert!(taller.height(f.centers[0]) >= f.height(f.centers[0]));
}

#[test]
fn no_antipodal_ghost() {
    let f = CloverField::new(9);
    for (i, &c) in f.centers.iter().enumerate() {
        assert!(tangent_uv(-c, c).is_none());
        for n in cap(-c, 0.5) {
            assert_eq!(f.inside(i, n), 0.0, "clover {i} seen on its far side");
        }
    }
    // Far from every center, only the seabed: under the sea.
    for n in icosphere(3).unwrap().positions {
        if f.centers.iter().all(|&c| angle(n, c) > 0.6) {
            assert!(f.height(n) < f.sea, "{n:?}");
        }
    }
}

#[test]
fn seed_rotates_centers() {
    let (a, b) = (CloverField::new(1), CloverField::new(2));
    let moved = a
        .centers
        .iter()
        .all(|&c| b.centers.iter().all(|&d| angle(c, d) > 0.05));
    assert!(moved, "{:?} {:?}", a.centers, b.centers);
    assert_eq!(CloverField::new(1), a, "same seed, same field");
    // Another seed is another geometry.
    assert_ne!(
        GeometryKey::new(0.866, &HeightField::Clover(a)),
        GeometryKey::new(0.866, &HeightField::Clover(b))
    );
    // The silhouette: inside at a lobe, outside between lobes and far away.
    assert!(clover_sdf((0.0, 0.15), 0.16, 0.14) < 0.0);
    assert!(clover_sdf((0.0, -0.25), 0.16, 0.14) < 0.0, "stem");
    assert!(clover_sdf((0.5, 0.5), 0.16, 0.14) > 0.0);
}

/// `P3_DUMP_DIR=<dir>` writes the clover planet (on body 1 of the project 3
/// system) facing the camera at about 120 px of radius, to check that the
/// silhouette reads as a clover.
#[test]
fn clover_frame_dump() {
    use rust_software_renderer::pipeline::{
        OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
    };
    use rust_software_renderer::rasterization::TriangleMode;
    use rust_software_renderer::rendering::Framebuffer;
    use rust_software_renderer::solar::system::SystemSpec;
    use rust_software_renderer::solar::{SolarMeshes, SolarScene};
    let Some(dir) = std::env::var_os("P3_DUMP_DIR") else {
        return;
    };
    let (w, h) = (800, 600);
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(w, h, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap();
    let f = CloverField::new(7);
    s.set_relief(1, HeightField::Clover(f)).unwrap();
    s.bodies[1].transform.yaw = 0.0;
    for (k, d) in [(0, 3.9f32), (1, 1.6)] {
        let c = f.centers[k % 4];
        let p = s.bodies[1].transform.position;
        s.ship.position = p + c * (d * s.radius_of(1));
        s.ship.forward = -c;
        s.ship.up = c.cross(Vector3::new(0.3, 1.0, 0.1)).cross(c).normalized() * -1.0;
        s.ship.position -= s.ship.up * 0.5;
        let mut fb = Framebuffer::new(w, h);
        s.render_frame(
            &mut fb,
            TriangleMode::FilledScanline,
            ShadedOptions::default(),
        )
        .unwrap();
        let rgb: Vec<u8> = fb
            .to_rgba_bytes()
            .chunks(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let path = std::path::Path::new(&dir).join(format!("p3_clover_{k}.ppm"));
        std::fs::write(
            path,
            [format!("P6 {w} {h} 255\n").as_bytes(), &rgb].concat(),
        )
        .unwrap();
    }
}

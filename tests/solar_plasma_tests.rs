//! External plasma loops of The King's Crown in the solar scene.

use rust_software_renderer::pipeline::face_normal;
use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::plasma_loops::{
    CROWN_ARCS, PlasmaLoops, SEGMENTS, SIDES, VERTICES_PER_ARC,
};

/// Below the nearest vertex (0.840) and the flattest face of `sphere.obj`.
const BURIED: f32 = 0.83;

fn angle(a: raylib::math::Vector3, b: raylib::math::Vector3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

#[test]
fn the_crown_has_a_handful_of_slender_arcs() {
    assert!((3..=8).contains(&CROWN_ARCS.len()));
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        assert!(a.span > 0.2 && a.span < 1.0, "{i}: span {}", a.span);
        assert!(a.height > 0.2 && a.height < 0.8, "{i}: height {}", a.height);
        assert!(a.width > 0.0 && a.width < a.height / 3.0, "{i}: width");
        assert!(a.intensity > 0.5 && a.intensity <= 1.5, "{i}: intensity");
        assert!(a.heat.abs() <= 0.3, "{i}: heat");
        assert!(a.latitude.abs() < 0.8, "{i}: away from the poles");
    }
}

#[test]
fn the_feet_lie_on_the_unit_sphere_half_a_span_from_the_center() {
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        let [f0, f1] = a.feet();
        assert!((f0.length() - 1.0).abs() < 1e-5 && (f1.length() - 1.0).abs() < 1e-5);
        assert!((angle(f0, f1) - a.span).abs() < 1e-4, "{i}: span");
        assert!(
            ((f0 + f1).normalized() - a.center()).length() < 1e-5,
            "{i}: center"
        );
    }
}

#[test]
fn the_arcs_spread_around_the_star_without_sharing_feet() {
    let north = CROWN_ARCS.iter().filter(|a| a.latitude > 0.0).count();
    assert!(north > 0 && north < CROWN_ARCS.len(), "both hemispheres");
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        for b in &CROWN_ARCS[i + 1..] {
            assert!(angle(a.center(), b.center()) > 0.6, "{i}: crowded");
            for fa in a.feet() {
                for fb in b.feet() {
                    assert!(angle(fa, fb) > 0.2, "{i}: shared foot");
                }
            }
        }
    }
}

#[test]
fn each_centerline_leaves_the_star_and_falls_back_in_its_own_plane() {
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        let normal = a.basis().normal;
        let [f0, f1] = a.feet();
        let (p0, p1) = (a.point(0.0, SPHERE_RADIUS), a.point(1.0, SPHERE_RADIUS));
        assert!(
            p0.length() < BURIED && p1.length() < BURIED,
            "{i}: feet buried"
        );
        assert!((p0.normalized() - f0).length() < 1e-5, "{i}: first foot");
        assert!((p1.normalized() - f1).length() < 1e-5, "{i}: second foot");
        let apex = a.point(0.5, SPHERE_RADIUS).length();
        assert!(
            (apex - SPHERE_RADIUS * (1.0 + a.height)).abs() < 1e-4,
            "{i}: apex {apex}"
        );
        for k in 0..=20 {
            let p = a.point(k as f32 / 20.0, SPHERE_RADIUS);
            assert!(p.dot(normal).abs() < 1e-5, "{i}: off its plane");
        }
    }
}

#[test]
fn the_tubes_are_small_closed_around_the_curve_and_face_outwards() {
    let loops = PlasmaLoops::crown();
    assert_eq!(loops.mesh.len(), CROWN_ARCS.len() * VERTICES_PER_ARC);
    assert!(
        loops.mesh.len() <= 8 * 600,
        "a few thousand corners at most"
    );
    for (i, a) in CROWN_ARCS.iter().enumerate() {
        let mesh = loops.arc_mesh(i);
        for (k, tri) in mesh.chunks(3).enumerate() {
            let segment = k / (2 * SIDES);
            let t = (segment as f32 + 0.5) / SEGMENTS as f32;
            let centroid = (tri[0].position + tri[1].position + tri[2].position) / 3.0;
            let n = face_normal(tri[0].position, tri[1].position, tri[2].position).unwrap();
            let axis = a.point(t, SPHERE_RADIUS);
            assert!(
                n.dot(centroid - axis) > 0.0,
                "{i}: triangle {k} faces inwards"
            );
        }
        // Ring `r` (t = r / SEGMENTS): the first corner of each quad of
        // segment `r`, or the third of segment `r - 1` for the last ring.
        let ring = |r: usize| -> Vec<f32> {
            (0..SIDES)
                .map(|j| match r {
                    SEGMENTS => mesh[((r - 1) * SIDES + j) * 6 + 2].position.length(),
                    _ => mesh[(r * SIDES + j) * 6].position.length(),
                })
                .collect()
        };
        assert!(ring(0).iter().all(|&r| r < BURIED), "{i}: first foot");
        assert!(ring(SEGMENTS).iter().all(|&r| r < BURIED), "{i}: last foot");
        let top = SPHERE_RADIUS * (1.0 + a.height - a.width);
        assert!(
            ring(SEGMENTS / 2).iter().all(|&r| r > top * 0.999),
            "{i}: apex outside the silhouette"
        );
        let reach = SPHERE_RADIUS * loops.extent();
        assert!(mesh.iter().all(|v| v.position.length() <= reach), "{i}");
    }
}

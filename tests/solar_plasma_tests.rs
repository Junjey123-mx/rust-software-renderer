//! External plasma loops of The King's Crown in the solar scene.

use raylib::math::Vector3;
use rust_software_renderer::celestial::{Body, CelestialMaterial};
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport, face_normal,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::plasma_loops::{
    CROWN_ARCS, PlasmaLoops, SEGMENTS, SIDES, VERTICES_PER_ARC,
};
use rust_software_renderer::solar::{
    BodyInstance, CameraMode, SPHERE_RADIUS, SolarMeshes, SolarScene,
};

/// Below the nearest vertex (0.840) and the flattest face of `sphere.obj`.
const BURIED: f32 = 0.83;

fn angle(a: Vector3, b: Vector3) -> f32 {
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

const W: u32 = 320;
const H: u32 = 240;

/// The crown alone at the origin under the orthographic front camera,
/// framed so the star with its loops fills most of the view.
fn star_scene(spin: f32, time: f32) -> SolarScene {
    let camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let view = OrbitView::for_world(camera, 2.0, Viewport::new(W, H, 0.1)).unwrap();
    let mut s = SolarScene::new(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 0.0, -1.0),
    );
    s.camera_mode = CameraMode::Orbit;
    s.bodies = vec![BodyInstance::new(
        Body::KingsCrown,
        Transform::new(Vector3::zero(), 1.3),
        spin,
    )];
    s.clock.time = time;
    s.update();
    s
}

fn frame(s: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb
}

/// Pixels whose depth the loops changed, given the frame without them.
fn loop_pixels(with: &Framebuffer, without: &Framebuffer) -> Vec<(i32, i32)> {
    let mut covered = Vec::new();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if with.get_depth(x, y) != without.get_depth(x, y) {
                covered.push((x, y));
            }
        }
    }
    covered
}

fn without_loops(mut s: SolarScene) -> SolarScene {
    s.plasma = PlasmaLoops::new(&[]);
    s
}

#[test]
fn the_loops_rise_outside_the_star_silhouette_in_warm_colors() {
    let s = star_scene(0.05, 6.0);
    let (with, without) = (frame(&s), frame(&without_loops(s.clone())));
    let covered = loop_pixels(&with, &without);
    let radius_px = s.radius_of(0) * s.view.scale();
    let (cx, cy) = (W as f32 / 2.0, H as f32 / 2.0);
    let outside: Vec<_> = covered
        .iter()
        .filter(|&&(x, y)| {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            (dx * dx + dy * dy).sqrt() > radius_px * 1.05
        })
        .collect();
    assert!(
        outside.len() > 200,
        "{} pixels outside the disk",
        outside.len()
    );
    // Warm: red leads green and green leads blue on average.
    let sum = outside.iter().fold(Vector3::zero(), |acc, &&(x, y)| {
        acc + with.get_pixel(x, y).unwrap()
    }) / outside.len() as f32;
    assert!(sum.x > sum.y && sum.y > sum.z && sum.x > 0.5, "{sum:?}");
}

#[test]
fn the_star_hides_the_far_side_of_its_loops() {
    let s = star_scene(0.05, 6.0);
    let options = ShadedOptions::default();
    let mut alone = Framebuffer::new(W, H);
    alone.clear();
    let free: usize = s
        .render_plasma_without_clear(&mut alone, TriangleMode::FilledScanline, options)
        .unwrap()
        .iter()
        .map(|a| a.fragments_shaded)
        .sum();
    let mut behind = Framebuffer::new(W, H);
    behind.clear();
    s.render_objects_without_clear(&mut behind, TriangleMode::FilledScanline, options)
        .unwrap();
    let occluded: usize = s
        .render_plasma_without_clear(&mut behind, TriangleMode::FilledScanline, options)
        .unwrap()
        .iter()
        .map(|a| a.fragments_shaded)
        .sum();
    assert!(occluded > 0 && occluded < free, "{occluded} of {free}");
}

#[test]
fn the_loops_stay_put_on_a_still_star_and_follow_its_spin() {
    let still = |time| {
        let s = star_scene(0.0, time);
        loop_pixels(&frame(&s), &frame(&without_loops(s.clone())))
    };
    // Only the colors animate: the covered pixels never move.
    let (a, b) = (star_scene(0.0, 3.0), star_scene(0.0, 7.0));
    assert_eq!(still(3.0), still(7.0));
    assert_ne!(frame(&a).to_rgba_bytes(), frame(&b).to_rgba_bytes());
    // The same instant on a turning star: the loops moved with it.
    let spun = star_scene(0.05, 7.0);
    assert_ne!(
        loop_pixels(&frame(&spun), &frame(&without_loops(spun.clone()))),
        still(7.0)
    );
}

#[test]
fn a_scene_without_the_crown_or_without_arcs_draws_no_loops() {
    let mut s = star_scene(0.05, 6.0);
    assert_eq!(s.plasma_host(), Some(0));
    s.bodies[0] = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.3),
        0.05,
    );
    assert_eq!(s.plasma_host(), None);
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let options = ShadedOptions::default();
    assert!(
        s.render_plasma_without_clear(&mut fb, TriangleMode::FilledScanline, options)
            .unwrap()
            .is_empty()
    );
    assert_eq!(without_loops(star_scene(0.05, 6.0)).plasma_host(), None);
}

#[test]
fn the_solar_crown_hands_its_surface_loops_to_the_external_arcs() {
    let lab6 = CelestialMaterial::for_body(Body::KingsCrown);
    assert_eq!(lab6.get("loops"), Some(1.0), "laboratory six unchanged");
    let s = star_scene(0.05, 0.0);
    let mut solar = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        s.view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    assert_eq!(solar.bodies[0].material.get("loops"), Some(0.0));
    assert_eq!(solar.plasma_host(), Some(0));
    solar
        .apply_overrides(&[("crown.loops".to_string(), 1.0)])
        .unwrap();
    assert_eq!(solar.bodies[0].material.get("loops"), Some(1.0));
}

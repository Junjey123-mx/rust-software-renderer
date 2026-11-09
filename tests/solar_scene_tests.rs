//! SolarScene: several instances, one framebuffer, one depth buffer.

use raylib::math::Vector3;
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    ModelBounds, OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport,
    direction_from_angles,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::{BodyInstance, Orbit, SolarMeshes, SolarScene};

const W: u32 = 320;
const H: u32 = 240;

fn fnv(fb: &Framebuffer) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in fb.to_rgba_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Front preset: the eye sits on -Z looking towards +Z (smaller z is nearer)
/// and its right vector is world -X, so world +X appears on the screen left.
/// World radius 4 in a 240 px viewport gives 24 px per unit.
fn scene() -> SolarScene {
    let cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let view = OrbitView::for_world(cam, 4.0, Viewport::new(W, H, 0.1)).unwrap();
    let light = direction_from_angles((-75.0f32).to_radians(), 20.0f32.to_radians());
    SolarScene::new(SolarMeshes::load().unwrap(), view, light)
}

fn options() -> ShadedOptions {
    ShadedOptions {
        threads: 1,
        ..ShadedOptions::default()
    }
}

fn body(b: Body, x: f32, z: f32, scale: f32) -> BodyInstance {
    BodyInstance::new(b, Transform::new(Vector3::new(x, 0.0, z), scale), 0.0)
}

fn draw(scene: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    scene
        .render_objects_without_clear(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    fb
}

fn pixel(fb: &Framebuffer, x: i32, y: i32) -> Vector3 {
    fb.get_pixel(x, y).unwrap()
}

#[test]
fn two_and_three_bodies_are_visible_at_once() {
    let mut s = scene();
    s.bodies = vec![
        body(Body::KingsCrown, 2.2, 0.0, 1.2),
        body(Body::HouseOfCards, -2.2, 0.0, 1.0),
    ];
    let fb = draw(&s);
    let bg = fb.background_color();
    let (cy, cx) = (H as i32 / 2, W as i32 / 2);
    assert_ne!(pixel(&fb, cx - 53, cy), bg, "crown on the left");
    assert_ne!(pixel(&fb, cx + 53, cy), bg, "house on the right");
    assert_eq!(pixel(&fb, cx, cy), bg, "gap in the middle");

    s.bodies.push(body(Body::CrimsonEntrance, 0.0, 0.0, 0.6));
    let fb = draw(&s);
    assert_ne!(pixel(&fb, cx, cy), bg, "entrance fills the gap");
    assert!(fb.count_non_background() > 3000);
}

#[test]
fn occlusion_does_not_depend_on_draw_order() {
    let far = body(Body::KingsCrown, 0.0, 2.0, 1.4);
    let near = body(Body::CrimsonEntrance, 0.0, -1.0, 0.6);
    let mut ab = scene();
    ab.bodies = vec![far, near];
    let mut ba = scene();
    ba.bodies = vec![near, far];
    let (fa, fb) = (draw(&ab), draw(&ba));
    assert_eq!(fnv(&fa), fnv(&fb));
    // The nearer entrance owns the center and the far crown stays visible around it.
    let (cy, cx) = (H as i32 / 2, W as i32 / 2);
    let mut only_near = scene();
    only_near.bodies = vec![near];
    assert_eq!(pixel(&fa, cx, cy), pixel(&draw(&only_near), cx, cy));
    assert_ne!(pixel(&fa, cx + 25, cy), fa.background_color());
    assert!(
        fa.get_depth(cx, cy).unwrap() < 0.0,
        "near side of the target"
    );
}

#[test]
fn drawing_never_clears_and_respects_a_nearer_existing_pixel() {
    let mut s = scene();
    s.bodies = vec![body(Body::HouseOfCards, 0.0, 0.0, 1.0)];
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let (cy, cx) = (H as i32 / 2, W as i32 / 2);
    let marker = Vector3::new(0.1, 0.9, 0.2);
    // A pixel far outside the body and one nearer than the body at its center.
    fb.point(2, 2, marker);
    fb.point_depth(cx, cy, marker, -100.0);
    s.render_objects_without_clear(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    assert_eq!(
        fb.get_pixel(2, 2),
        Some(marker),
        "untouched region survives"
    );
    assert_eq!(fb.get_pixel(cx, cy), Some(marker), "nearer pixel wins");
    assert_ne!(fb.get_pixel(cx + 10, cy), Some(fb.background_color()));
}

#[test]
fn changing_one_instance_leaves_the_other_pixels_identical() {
    let mut s = scene();
    s.bodies = vec![
        body(Body::KingsCrown, 2.2, 0.0, 1.0),
        body(Body::HouseOfCards, -2.2, 0.0, 1.0),
    ];
    let before = draw(&s);
    s.bodies[1].transform.scale = 0.7;
    let after = draw(&s);
    let (cy, cx) = (H as i32 / 2, W as i32 / 2);
    // Left body unchanged pixel by pixel on its half of the screen.
    for y in 0..H as i32 {
        for x in 0..cx {
            assert_eq!(pixel(&before, x, y), pixel(&after, x, y), "({x}, {y})");
        }
    }
    assert_ne!(fnv(&before), fnv(&after));
    assert_eq!(pixel(&after, cx + 53, cy + 18), after.background_color());
}

#[test]
fn world_view_matches_a_model_view_of_the_same_radius_and_rejects_bad_radii() {
    let cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    let vp = Viewport::new(W, H, 0.1);
    let by_world = OrbitView::for_world(cam, 4.4, vp).unwrap();
    let by_model = OrbitView::new(
        cam,
        &ModelBounds {
            center: Vector3::zero(),
            radius: 4.4,
        },
        vp,
    )
    .unwrap();
    assert_eq!(by_world, by_model);
    // 240 px * 0.8 / (2 * 4.4) pixels per world unit.
    assert!((by_world.scale() - 240.0 * 0.8 / 8.8).abs() < 1e-4);
    assert!(OrbitView::for_world(cam, 0.0, vp).is_err());
    assert!(OrbitView::for_world(cam, f32::NAN, vp).is_err());
}

#[test]
fn world_view_frames_every_body_of_a_radius_4_4_system_without_refitting() {
    let mut s = scene();
    s.view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Top),
        4.4 + 0.9,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    s.bodies = vec![
        body(Body::KingsCrown, 0.0, 0.0, 1.3),
        body(Body::CrimsonEntrance, 2.6, 0.0, 0.6),
        body(Body::HouseOfCards, -4.4, 0.0, 0.9),
    ];
    let stats = {
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        s.render_objects_without_clear(&mut fb, TriangleMode::FilledScanline, options())
            .unwrap()
    };
    assert!(stats.iter().all(|st| st.fragments_shaded > 50), "{stats:?}");
    // Rotating the camera never changes the scale (no per-instance fit).
    let before = s.view.scale();
    s.view.camera.orbit(1.0);
    assert_eq!(s.view.scale(), before);
}

fn orbiting_system() -> SolarScene {
    let mut s = scene();
    s.bodies = vec![
        body(Body::KingsCrown, 0.0, 0.0, 1.3),
        body(Body::CrimsonEntrance, 0.0, 0.0, 0.6).with_orbit(Orbit::new(2.6, 0.5, 0.0).unwrap()),
        body(Body::HouseOfCards, 0.0, 0.0, 0.9)
            .with_orbit(Orbit::new(3.2, 0.2, std::f32::consts::PI).unwrap()),
    ];
    s.bodies[1].spin_speed = 0.6;
    s
}

#[test]
fn update_places_orbiting_bodies_and_keeps_the_star_at_the_center() {
    let mut s = orbiting_system();
    s.advance(0.0);
    assert_eq!(s.bodies[0].transform.position, Vector3::zero());
    assert!((s.bodies[1].transform.position - Vector3::new(2.6, 0.0, 0.0)).length() < 1e-5);
    assert!((s.bodies[2].transform.position - Vector3::new(-3.2, 0.0, 0.0)).length() < 1e-5);
    for _ in 0..60 {
        s.advance(1.0 / 60.0);
    }
    assert!((s.time() - 1.0).abs() < 1e-4);
    assert_eq!(
        s.bodies[0].transform.position,
        Vector3::zero(),
        "star never moves"
    );
    let p = s.bodies[1].transform.position;
    assert!((p.length() - 2.6).abs() < 1e-4 && p.z > 0.0, "{p:?}");
    assert!(
        (s.bodies[1].transform.yaw - 0.6).abs() < 1e-4,
        "spin = speed * time"
    );
}

#[test]
fn the_same_simulated_time_gives_the_same_image_regardless_of_frame_rate() {
    let mut a = orbiting_system();
    let mut b = orbiting_system();
    for _ in 0..30 {
        a.advance(1.0 / 30.0);
    }
    for _ in 0..60 {
        b.advance(1.0 / 60.0);
    }
    assert!((a.time() - b.time()).abs() < 1e-4);
    // Snap both to exactly the same time so rounding cannot leak into pixels.
    a.clock.time = 1.0;
    b.clock.time = 1.0;
    a.update();
    b.update();
    assert_eq!(fnv(&draw(&a)), fnv(&draw(&b)));
}

#[test]
fn pause_freezes_positions_spins_and_the_material_time() {
    let mut s = orbiting_system();
    s.advance(0.5);
    let before = draw(&s);
    let positions: Vec<_> = s.bodies.iter().map(|b| b.transform).collect();
    s.clock.toggle_pause();
    for _ in 0..10 {
        s.advance(0.1);
    }
    let after: Vec<_> = s.bodies.iter().map(|b| b.transform).collect();
    assert_eq!(positions, after);
    assert_eq!(fnv(&before), fnv(&draw(&s)));
    s.clock.toggle_pause();
    s.advance(0.1);
    assert_ne!(
        positions,
        s.bodies.iter().map(|b| b.transform).collect::<Vec<_>>()
    );
}

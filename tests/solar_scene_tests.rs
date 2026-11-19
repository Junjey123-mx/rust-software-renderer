//! SolarScene: several instances, one framebuffer, one depth buffer.

use raylib::math::Vector3;
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    ModelBounds, OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport,
    direction_from_angles,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::viewer::step;
use rust_software_renderer::solar::{BodyInstance, CameraMode, Orbit, SolarMeshes, SolarScene};

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
    let mut s = SolarScene::new(SolarMeshes::load().unwrap(), view, light);
    s.camera_mode = CameraMode::Orbit;
    s
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

#[test]
fn planets_are_lit_from_the_star_and_the_star_keeps_the_scene_light() {
    let mut s = orbiting_system();
    s.advance(0.0);
    let star = &s.bodies[0];
    assert_eq!(s.light_for(star), s.light_world);
    // Entrance sits at +2.6 X: its light points towards -X, unit length.
    let l = s.light_for(&s.bodies[1]);
    assert!((l - Vector3::new(-1.0, 0.0, 0.0)).length() < 1e-5, "{l:?}");
    s.advance(1.0);
    let p = s.bodies[2].transform.position;
    let l = s.light_for(&s.bodies[2]);
    assert!((l.length() - 1.0).abs() < 1e-5);
    assert!(l.dot(-p) > 0.999, "points at the star from {p:?}");
}

#[test]
fn the_hemisphere_facing_the_star_is_brighter() {
    // House at +3 X with no spin. In the Front view world +X is the screen
    // left, so the star-facing side (-X) is the right half of its disk.
    let mut s = scene();
    s.bodies = vec![
        body(Body::KingsCrown, 0.0, 0.0, 0.5),
        body(Body::HouseOfCards, 3.0, 0.0, 1.0),
    ];
    let fb = draw(&s);
    let (cy, cx) = (H as i32 / 2, W as i32 / 2 - 72); // 3 units * 24 px
    let lum = |x: i32| {
        let p = pixel(&fb, x, cy);
        p.x + p.y + p.z
    };
    let towards_star: f32 = (6..18).map(|d| lum(cx + d)).sum();
    let away: f32 = (6..18).map(|d| lum(cx - d)).sum();
    assert!(towards_star > away * 1.5, "{towards_star} vs {away}");

    // Rotating the planet half a turn keeps the lit side towards the star.
    s.bodies[1].transform.yaw = std::f32::consts::PI;
    let fb2 = draw(&s);
    let lum2 = |x: i32| {
        let p = pixel(&fb2, x, cy);
        p.x + p.y + p.z
    };
    let towards2: f32 = (6..18).map(|d| lum2(cx + d)).sum();
    let away2: f32 = (6..18).map(|d| lum2(cx - d)).sum();
    assert!(towards2 > away2 * 1.5, "{towards2} vs {away2}");
}

#[test]
fn perspective_mode_grows_the_nearer_body_and_orbit_mode_does_not() {
    // Front rig: the eye sits on -Z, so z = -2 is nearer than z = +2.
    let fragments = |mode: CameraMode| {
        let mut s = scene();
        s.camera_mode = mode;
        s.rig = CameraRig::Orbit;
        s.bodies = vec![
            body(Body::CrimsonEntrance, 1.5, -2.0, 0.6),
            body(Body::HouseOfCards, -1.5, 2.0, 0.6),
        ];
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        let st = s
            .render_objects_without_clear(&mut fb, TriangleMode::FilledScanline, options())
            .unwrap();
        (st[0].fragments_shaded as f32, st[1].fragments_shaded as f32)
    };
    let (near, far) = fragments(CameraMode::Perspective);
    assert!(near > far * 1.3, "perspective: {near} vs {far}");
    let (a, b) = fragments(CameraMode::Orbit);
    assert!((a - b).abs() < a * 0.05, "orbit: {a} vs {b}");
}

#[test]
fn perspective_camera_follows_the_orbit_rig_and_zoom_approaches() {
    let mut s = orbiting_system();
    s.camera_mode = CameraMode::Perspective;
    let far = s.perspective_camera();
    assert_eq!(far.target, Vector3::zero());
    assert!((far.eye.normalized() - s.view.camera.eye_direction()).length() < 1e-5);
    s.view.camera.zoom_by(2.0);
    let near = s.perspective_camera();
    assert!((near.eye.length() - far.eye.length() / 2.0).abs() < 1e-4);
    s.view.camera.orbit(0.7);
    assert!(
        (s.perspective_camera().eye.normalized() - s.view.camera.eye_direction()).length() < 1e-5
    );
}

fn flying_scene() -> SolarScene {
    let mut s = orbiting_system();
    s.camera_mode = CameraMode::Perspective;
    s.rig = CameraRig::Follow;
    s.advance(0.0);
    s
}

#[test]
fn the_frame_draws_the_bodies_and_the_spacecraft_through_the_follow_camera() {
    let s = flying_scene();
    let mut fb = Framebuffer::new(W, H);
    let shaded = s
        .render_frame(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    assert!(shaded > 0);
    // The ship sits just ahead and below the follow camera's view center.
    let ship_px = s.projection();
    let rust_software_renderer::pipeline::ViewProjection::Perspective(cam) = ship_px else {
        panic!("follow rig is perspective");
    };
    let p = cam
        .project_view(cam.world_to_view(s.ship.position), W, H)
        .unwrap();
    let px = fb.get_pixel(p.x as i32, p.y as i32).unwrap();
    assert_ne!(px, fb.background_color(), "spacecraft visible at {p:?}");
    // Bodies are visible ahead too.
    let mut bodies_only = Framebuffer::new(W, H);
    bodies_only.clear();
    s.render_objects_without_clear(&mut bodies_only, TriangleMode::FilledScanline, options())
        .unwrap();
    assert!(bodies_only.count_non_background() > 200);
}

#[test]
fn render_frame_clears_once_so_old_pixels_never_survive() {
    let s = flying_scene();
    let mut fb = Framebuffer::new(W, H);
    fb.point_depth(1, 1, Vector3::one(), -1.0e6);
    s.render_frame(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    // The stale pixel is gone: the corner shows what a fresh frame shows
    // there (the sky backdrop).
    let mut fresh = Framebuffer::new(W, H);
    s.render_frame(&mut fresh, TriangleMode::FilledScanline, options())
        .unwrap();
    assert_ne!(fb.get_pixel(1, 1), Some(Vector3::one()));
    assert_eq!(fb.get_pixel(1, 1), fresh.get_pixel(1, 1));
}

#[test]
fn follow_controls_fly_the_ship_and_the_camera_follows_while_orbit_rig_moves_the_camera() {
    let mut s = flying_scene();
    let start = s.ship.position;
    for _ in 0..20 {
        step(&mut s, &[SolarAction::Forward], &[], Look::default(), 0.05);
    }
    assert!((s.ship.position - start).length() > 1.0, "flew forward");
    let cam_eye = match s.projection() {
        rust_software_renderer::pipeline::ViewProjection::Perspective(c) => c.eye,
        _ => unreachable!(),
    };
    assert!(
        (cam_eye
            - (s.ship.position - s.ship.forward * s.follow.distance + s.ship.up * s.follow.height))
            .length()
            < 1e-4
    );

    // V switches to the orbit rig: thrust no longer moves the ship, the
    // mouse orbits the camera instead.
    step(
        &mut s,
        &[SolarAction::ToggleCamera],
        &[],
        Look::default(),
        0.0,
    );
    assert_eq!(s.rig, CameraRig::Orbit);
    let parked = s.ship.position;
    let (az, el) = (s.view.camera.azimuth, s.view.camera.elevation);
    let look = Look {
        yaw: 0.1,
        pitch: 0.05,
    };
    step(&mut s, &[SolarAction::Forward], &[], look, 0.05);
    assert_eq!(s.ship.position, parked);
    assert_ne!(s.view.camera.azimuth, az);
    assert_ne!(s.view.camera.elevation, el);
}

#[test]
fn flight_keeps_working_while_the_simulation_is_paused() {
    let mut s = flying_scene();
    step(
        &mut s,
        &[SolarAction::TogglePause],
        &[],
        Look::default(),
        0.0,
    );
    let planets: Vec<_> = s.bodies.iter().map(|b| b.transform).collect();
    let start = s.ship.position;
    for _ in 0..5 {
        step(&mut s, &[SolarAction::Forward], &[], Look::default(), 0.05);
    }
    assert_eq!(
        planets,
        s.bodies.iter().map(|b| b.transform).collect::<Vec<_>>()
    );
    assert!((s.ship.position - start).length() > 0.3);
}

#[test]
fn the_spawn_is_outside_every_body_and_the_ship_is_small() {
    let s = flying_scene();
    let r = rust_software_renderer::solar::SPHERE_RADIUS;
    for b in &s.bodies {
        let d = (s.ship.position - b.transform.position).length();
        assert!(d > b.transform.scale * r + s.ship.radius() + 1.0, "{d}");
    }
    assert!(
        s.ship.radius() < 0.6 * r,
        "smaller than the smallest planet"
    );
}

#[test]
fn the_scene_reuses_its_buffers_across_frames_without_changing_the_image() {
    let s = flying_scene();
    let mut a = Framebuffer::new(W, H);
    s.render_frame(&mut a, TriangleMode::FilledScanline, options())
        .unwrap();
    let capacity = s.scratch_capacity();
    assert!(capacity > 0);
    let mut b = Framebuffer::new(W, H);
    s.render_frame(&mut b, TriangleMode::FilledScanline, options())
        .unwrap();
    assert_eq!(fnv(&a), fnv(&b));
    assert_eq!(s.scratch_capacity(), capacity);
}

// ---------------------------------------------------------------------------
// Frustum culling
// ---------------------------------------------------------------------------

use rust_software_renderer::pipeline::{PerspectiveCamera, sphere_visible};

#[test]
fn sphere_visibility_rejects_behind_and_beside_but_keeps_partial_overlaps() {
    let cam = PerspectiveCamera::new(
        Vector3::zero(),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        std::f32::consts::FRAC_PI_2,
        0.1,
    )
    .unwrap();
    // 90 degree vertical fov, 4:3: the side planes are at x/z = 4/3, y/z = 1.
    assert!(sphere_visible(
        &cam,
        Vector3::new(0.0, 0.0, 5.0),
        1.0,
        400,
        300
    ));
    assert!(
        !sphere_visible(&cam, Vector3::new(0.0, 0.0, -5.0), 1.0, 400, 300),
        "behind"
    );
    assert!(
        sphere_visible(&cam, Vector3::new(0.0, 0.0, -0.5), 1.0, 400, 300),
        "around the eye"
    );
    assert!(
        !sphere_visible(&cam, Vector3::new(0.0, 20.0, 5.0), 1.0, 400, 300),
        "above"
    );
    // Center just outside the top plane (y/z = 1.1) but radius reaching in.
    assert!(sphere_visible(
        &cam,
        Vector3::new(0.0, 5.5, 5.0),
        1.0,
        400,
        300
    ));
    // Horizontally the view is wider: x/z = 1.2 is inside, 1.5 with r = 0.1 is out.
    assert!(sphere_visible(
        &cam,
        Vector3::new(6.0, 0.0, 5.0),
        0.1,
        400,
        300
    ));
    assert!(!sphere_visible(
        &cam,
        Vector3::new(7.5, 0.0, 5.0),
        0.1,
        400,
        300
    ));
}

#[test]
fn a_culled_body_could_not_have_added_a_single_fragment() {
    // Soundness: sweep a body across and behind the camera; whenever the
    // scene culls it, drawing it for real must shade nothing.
    let base = flying_scene();
    let mut state = 12345u32;
    let mut rand = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / 16_777_216.0 * 2.0 - 1.0
    };
    let mut culled = 0;
    let mut kept_near_edge = 0;
    for _ in 0..300 {
        let mut s = base.clone();
        let offset = Vector3::new(rand() * 6.0, rand() * 5.0, rand() * 6.0);
        s.bodies.truncate(1);
        s.bodies[0].orbit = None;
        s.bodies[0].transform.position = s.ship.position + offset;
        s.bodies[0].transform.scale = 0.3 + 0.3 * (rand() + 1.0);
        let visible = s.body_in_view(0, W, H);
        let cam = match s.projection() {
            rust_software_renderer::pipeline::ViewProjection::Perspective(c) => c,
            _ => unreachable!(),
        };
        let center_inside = cam
            .project_view(cam.world_to_view(s.bodies[0].transform.position), W, H)
            .is_some_and(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < W as f32 && p.y < H as f32);
        // Draw without the cull to see what it would really produce.
        let frame = s.frame_context(TriangleMode::FilledScanline, options());
        let frame = rust_software_renderer::solar::FrameContext {
            light_world: s.light_for(&s.bodies[0]),
            ..frame
        };
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        let st = rust_software_renderer::solar::render_instance(
            &mut fb,
            &s.meshes.sphere,
            &s.bodies[0],
            &frame,
        )
        .unwrap();
        if !visible {
            culled += 1;
            assert_eq!(
                st.fragments_shaded, 0,
                "culled body would shade at {offset:?}"
            );
        } else if !center_inside && st.fragments_shaded > 0 {
            kept_near_edge += 1;
        }
    }
    assert!(culled > 50, "the sweep exercises culling: {culled}");
    assert!(
        kept_near_edge > 0,
        "bodies with the center off screen are still drawn"
    );
}

#[test]
fn facing_empty_space_skips_every_body_and_the_image_is_unchanged() {
    let mut s = flying_scene();
    s.ship.yaw(std::f32::consts::PI);
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let stats = s
        .render_objects_without_clear(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    assert!(
        stats.iter().all(|st| st.triangles_processed == 0),
        "{stats:?}"
    );
    assert_eq!(fb.count_non_background(), 0);
    // Orthographic orbit camera: a body far off screen is skipped, one in
    // view is drawn.
    let mut o = scene();
    o.bodies = vec![
        body(Body::HouseOfCards, 30.0, 0.0, 1.0),
        body(Body::KingsCrown, 0.0, 0.0, 1.0),
    ];
    assert!(!o.body_in_view(0, W, H) && o.body_in_view(1, W, H));
}

// ---------------------------------------------------------------------------
// Frame batch (one shading pool per frame)
// ---------------------------------------------------------------------------

#[test]
fn the_frame_batch_matches_drawing_the_instances_one_by_one() {
    use rust_software_renderer::benchmark::SolarBenchConfig;
    use rust_software_renderer::benchmark::solar::scenario_scene;
    let config = SolarBenchConfig {
        width: W,
        height: H,
        ..SolarBenchConfig::default()
    };
    let one_by_one = |s: &SolarScene, mode: TriangleMode, opts: ShadedOptions| {
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        let sky = s.sky_camera();
        rust_software_renderer::solar::nebula::draw_nebula(&mut fb, &s.nebula, &sky);
        rust_software_renderer::solar::skybox::draw_stars(&mut fb, &s.stars, &sky);
        s.render_objects_without_clear(&mut fb, mode, opts).unwrap();
        s.render_plasma_without_clear(&mut fb, mode, opts).unwrap();
        s.render_effects_without_clear(&mut fb, mode, opts).unwrap();
        s.render_ship_without_clear(&mut fb, mode, opts).unwrap();
        fnv(&fb)
    };
    let mut checked = 0;
    for id in ["C", "D", "E", "F", "G", "J"] {
        for camera in [CameraMode::Perspective, CameraMode::Orbit] {
            let mut s = scenario_scene(id, &config).unwrap();
            s.camera_mode = camera;
            for mode in [
                TriangleMode::FilledScanline,
                TriangleMode::FilledBarycentric,
            ] {
                for threads in [1, 0] {
                    let opts = ShadedOptions {
                        threads,
                        ..ShadedOptions::default()
                    };
                    let mut fb = Framebuffer::new(W, H);
                    s.render_frame(&mut fb, mode, opts).unwrap();
                    assert_eq!(
                        fnv(&fb),
                        one_by_one(&s, mode, opts),
                        "{id} {camera:?} {mode} {threads}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 48);
}

#[test]
fn the_batch_shades_no_more_fragments_than_one_by_one_and_wireframe_still_draws() {
    let s = flying_scene();
    let mut fb = Framebuffer::new(W, H);
    let batched = s
        .render_frame(&mut fb, TriangleMode::FilledScanline, options())
        .unwrap();
    let mut one = Framebuffer::new(W, H);
    one.clear();
    let bodies = s
        .render_objects_without_clear(&mut one, TriangleMode::FilledScanline, options())
        .unwrap();
    let plasma = s
        .render_plasma_without_clear(&mut one, TriangleMode::FilledScanline, options())
        .unwrap();
    let effects = s
        .render_effects_without_clear(&mut one, TriangleMode::FilledScanline, options())
        .unwrap();
    let ship = s
        .render_ship_without_clear(&mut one, TriangleMode::FilledScanline, options())
        .unwrap();
    let separate: usize = bodies
        .iter()
        .chain(&plasma)
        .chain(&effects)
        .map(|b| b.fragments_shaded)
        .sum::<usize>()
        + ship.fragments_shaded;
    assert!(
        batched > 0 && batched <= separate,
        "{batched} vs {separate}"
    );
    let mut wire = Framebuffer::new(W, H);
    assert_eq!(
        s.render_frame(&mut wire, TriangleMode::Wireframe, options())
            .unwrap(),
        0
    );
    assert!(wire.count_non_background() > 100);
}

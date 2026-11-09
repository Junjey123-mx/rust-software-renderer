//! Per-instance shader uniforms in object space.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::Transform;
use rust_software_renderer::solar::instance_uniforms;
use std::f32::consts::FRAC_PI_2;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-5
}

#[test]
fn identity_instance_keeps_world_directions() {
    let light = Vector3::new(0.0, 0.0, 1.0);
    let eye = Vector3::new(1.0, 0.0, 0.0);
    let u = instance_uniforms(&Transform::default(), 4.5, light, eye);
    assert_eq!(u.time, 4.5);
    assert!(close(u.light_direction, light));
    assert!(close(u.eye_direction, eye));
}

#[test]
fn rotated_instance_receives_directions_in_its_own_space() {
    // Yaw +90 degrees turns object -Z to world -X, so world -X is object -Z.
    let t = Transform {
        yaw: FRAC_PI_2,
        ..Transform::default()
    };
    let u = instance_uniforms(
        &t,
        0.0,
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    assert!(
        close(u.light_direction, Vector3::new(0.0, 0.0, -1.0)),
        "{:?}",
        u.light_direction
    );
    assert!(close(u.eye_direction, Vector3::new(0.0, 1.0, 0.0)));
}

#[test]
fn light_towards_the_star_is_unit_and_translation_independent() {
    let planet = Transform::new(Vector3::new(3.0, 0.0, 4.0), 0.6);
    let light_world = Vector3::zero() - planet.position;
    let u = instance_uniforms(&planet, 1.0, light_world, Vector3::new(0.0, 0.0, 1.0));
    assert!((u.light_direction.length() - 1.0).abs() < 1e-5);
    assert!(close(u.light_direction, Vector3::new(-0.6, 0.0, -0.8)));
}

#[test]
fn zero_direction_stays_zero_instead_of_nan() {
    let u = instance_uniforms(&Transform::default(), 0.0, Vector3::zero(), Vector3::zero());
    assert_eq!(u.light_direction, Vector3::zero());
    assert_eq!(u.eye_direction, Vector3::zero());
}

// ---------------------------------------------------------------------------
// BodyInstance + render_instance
// ---------------------------------------------------------------------------

use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    ModelBounds, OrbitCamera, OrbitView, ShadedOptions, ViewPreset, ViewProjection, Viewport,
    direction_from_angles,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::{BodyInstance, FrameContext, SolarMeshes, render_instance};

const W: u32 = 320;
const H: u32 = 240;

fn world_view(azimuth: f32) -> OrbitView {
    let mut cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    cam.azimuth = azimuth;
    let world = ModelBounds {
        center: Vector3::zero(),
        radius: 3.0,
    };
    OrbitView::new(cam, &world, Viewport::new(W, H, 0.1)).unwrap()
}

fn light() -> Vector3 {
    direction_from_angles((-75.0f32).to_radians(), 20.0f32.to_radians())
}

fn draw(inst: &BodyInstance, view: OrbitView, light_world: Vector3) -> (Framebuffer, usize) {
    let meshes = SolarMeshes::load().unwrap();
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let frame = FrameContext {
        projection: ViewProjection::Orbit(view),
        time: 6.0,
        light_world,
        mode: TriangleMode::FilledScanline,
        options: ShadedOptions {
            threads: 1,
            ..ShadedOptions::default()
        },
    };
    let stats = render_instance(&mut fb, &meshes.sphere, inst, &frame).unwrap();
    (fb, stats.fragments_shaded)
}

#[test]
fn translated_instance_leaves_the_center_empty_and_scale_changes_coverage() {
    let centered = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.0),
        0.0,
    );
    let (fb, n1) = draw(&centered, world_view(0.0), light());
    let bg = fb.background_color();
    assert_ne!(fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap(), bg);

    let moved = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::new(2.0, 0.0, 0.0), 1.0),
        0.0,
    );
    let (fb, n2) = draw(&moved, world_view(0.0), light());
    assert_eq!(fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap(), bg);
    assert!(
        (n1 as i64 - n2 as i64).abs() < n1 as i64 / 20,
        "same size: {n1} vs {n2}"
    );

    let bigger = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.5),
        0.0,
    );
    let (_, n3) = draw(&bigger, world_view(0.0), light());
    assert!(n3 > n1 * 2, "1.5x scale covers > 2x pixels: {n3} vs {n1}");
}

#[test]
fn pattern_stays_attached_when_body_camera_and_light_rotate_together() {
    // Rotating the body by yaw, the camera by the same azimuth and the light
    // by the same angle leaves the relative geometry unchanged, so the image
    // must be the same up to rasterization rounding.
    let still = BodyInstance::new(
        Body::CrimsonEntrance,
        Transform::new(Vector3::zero(), 1.0),
        0.0,
    );
    let (a, _) = draw(&still, world_view(0.0), light());
    let yaw = 0.9f32;
    let turned = BodyInstance {
        transform: Transform {
            yaw,
            ..still.transform
        },
        ..still
    };
    let (s, c) = yaw.sin_cos();
    let l = light();
    let light_turned = Vector3::new(l.x * c + l.z * s, l.y, -l.x * s + l.z * c);
    let (b, _) = draw(&turned, world_view(yaw), light_turned);
    let mut differing = 0;
    let mut body_pixels = 0;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let (pa, pb) = (a.get_pixel(x, y).unwrap(), b.get_pixel(x, y).unwrap());
            if pa != a.background_color() {
                body_pixels += 1;
            }
            if (pa - pb).length() > 0.05 {
                differing += 1;
            }
        }
    }
    assert!(body_pixels > 1000);
    assert!(
        differing * 100 < body_pixels * 2,
        "{differing} of {body_pixels} body pixels differ"
    );

    // And with the camera fixed, the rotated pattern is visibly different.
    let (c_fb, _) = draw(&turned, world_view(0.0), light());
    assert_ne!(
        c_fb.get_pixel(W as i32 / 2, H as i32 / 2),
        a.get_pixel(W as i32 / 2, H as i32 / 2)
    );
}

#[test]
fn render_instance_does_not_clear_the_framebuffer() {
    let meshes = SolarMeshes::load().unwrap();
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let marker = Vector3::new(0.1, 0.9, 0.2);
    fb.point(3, 3, marker);
    let inst = BodyInstance::new(Body::KingsCrown, Transform::new(Vector3::zero(), 1.0), 0.0);
    let frame = FrameContext {
        projection: ViewProjection::Orbit(world_view(0.0)),
        time: 0.0,
        light_world: light(),
        mode: TriangleMode::FilledScanline,
        options: ShadedOptions {
            threads: 1,
            ..ShadedOptions::default()
        },
    };
    render_instance(&mut fb, &meshes.sphere, &inst, &frame).unwrap();
    assert_eq!(fb.get_pixel(3, 3), Some(marker));
}

// ---------------------------------------------------------------------------
// Opt-in adaptive quality
// ---------------------------------------------------------------------------

use rust_software_renderer::celestial::CelestialMaterial;
use rust_software_renderer::solar::instance::{FULL_DETAIL_PX, LOW_DETAIL_PX};
use rust_software_renderer::solar::{Quality, adapted_material};

#[test]
fn full_quality_and_large_bodies_keep_the_material_unchanged() {
    for body in Body::ALL {
        let m = CelestialMaterial::for_body(body);
        for px in [0.0, 5.0, 50.0, 500.0, f32::INFINITY] {
            assert_eq!(adapted_material(&m, Quality::Full, px), m, "{body} {px}");
        }
        assert_eq!(adapted_material(&m, Quality::Adaptive, FULL_DETAIL_PX), m);
        assert_eq!(adapted_material(&m, Quality::Adaptive, f32::INFINITY), m);
    }
}

#[test]
fn adaptive_quality_ramps_detail_down_on_small_bodies_only() {
    let m = CelestialMaterial::for_body(Body::CrimsonEntrance);
    let steps = |px: f32| {
        adapted_material(&m, Quality::Adaptive, px)
            .get("parallax_steps")
            .unwrap()
    };
    let octaves = |px: f32| {
        adapted_material(&m, Quality::Adaptive, px)
            .get("terrain_octaves")
            .unwrap()
    };
    assert_eq!(steps(LOW_DETAIL_PX), 2.0);
    assert_eq!(octaves(1.0), 3.0);
    assert_eq!(
        steps(FULL_DETAIL_PX - 0.01),
        8.0,
        "a hair below full keeps the full count"
    );
    // Monotonic: a larger body never gets less detail.
    let mut last = 0.0;
    for px in (0..140).map(|i| i as f32) {
        let s = steps(px);
        assert!(s >= last, "{px}");
        last = s;
    }
    let c = CelestialMaterial::for_body(Body::KingsCrown);
    assert_eq!(
        adapted_material(&c, Quality::Adaptive, 10.0).get("plasma_octaves"),
        Some(3.0)
    );
    let h = CelestialMaterial::for_body(Body::HouseOfCards);
    assert_eq!(
        adapted_material(&h, Quality::Adaptive, 10.0),
        h,
        "house has no cost knob here"
    );
    assert!("adaptive".parse::<Quality>().is_ok() && "ultra".parse::<Quality>().is_err());
}

#[test]
fn projected_radius_shrinks_with_distance() {
    use rust_software_renderer::pipeline::PerspectiveCamera;
    use rust_software_renderer::solar::projected_radius_px;
    let cam = PerspectiveCamera::new(
        Vector3::zero(),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        std::f32::consts::FRAC_PI_2,
        0.05,
    )
    .unwrap();
    let p = ViewProjection::Perspective(cam);
    let near = projected_radius_px(&p, Vector3::new(0.0, 0.0, 5.0), 1.0, 600);
    let far = projected_radius_px(&p, Vector3::new(0.0, 0.0, 10.0), 1.0, 600);
    assert!((near - 60.0).abs() < 1e-3 && (far - 30.0).abs() < 1e-3);
    assert!(projected_radius_px(&p, Vector3::new(0.0, 0.0, 0.5), 1.0, 600).is_infinite());
}

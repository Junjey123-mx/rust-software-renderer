//! Planet atmospheres (`solar::atmosphere`): a halo around the disk that
//! is brightest on the day side, a rim that grows towards the limb, haze
//! from inside, hidden by whatever stands in front, each planet with its
//! own air, and only at the Grand scale of the live viewer.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, PerspectiveCamera, ViewPreset, Viewport,
};
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::atmosphere::{Shell, air_of, draw_atmospheres};
use rust_software_renderer::solar::{FOV_Y, SolarMeshes, SolarScene};

const W: u32 = 160;
const H: u32 = 120;

fn clover_shell(sun: Vector3) -> Shell {
    Shell {
        center: Vector3::zero(),
        ground: 5.0,
        top: 6.5,
        sun,
        air: air_of("Clover Haven").unwrap(),
    }
}

/// A frame of a black planet of `shell.ground` seen from `eye`: its depth
/// written where the pixel's ray meets the sphere, nothing elsewhere.
fn frame(camera: &PerspectiveCamera, shell: &Shell) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    let b = camera.basis();
    let focal = camera.focal_px(H);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let ray = b.right * ((x as f32 + 0.5 - W as f32 / 2.0) / focal)
                + b.up * ((H as f32 / 2.0 - y as f32 - 0.5) / focal)
                + b.forward;
            let d = ray.normalized();
            let oc = shell.center - camera.eye;
            let tc = oc.dot(d);
            let b2 = oc.dot(oc) - tc * tc;
            let r2 = shell.ground * shell.ground;
            if b2 < r2 {
                let t = tc - (r2 - b2).sqrt();
                fb.set_depth(x, y, t / ray.length());
            }
        }
    }
    fb
}

fn camera(eye: Vector3, target: Vector3) -> PerspectiveCamera {
    PerspectiveCamera::new(eye, target, Vector3::new(0.0, 1.0, 0.0), FOV_Y, 0.05).unwrap()
}

fn brightness(fb: &Framebuffer, x: i32, y: i32) -> f32 {
    let c = fb.get_pixel(x, y).unwrap();
    c.x + c.y + c.z
}

/// Column of the first pixel of row `y` (from the left edge) the planet's
/// depth covers.
fn left_limb(fb: &Framebuffer, y: i32) -> i32 {
    (0..W as i32)
        .find(|&x| fb.get_depth(x, y).is_some_and(f32::is_finite))
        .unwrap()
}

#[test]
fn every_planet_has_its_own_air_and_the_star_none() {
    let air = |n| air_of(n).unwrap();
    let (crimson, clover, hearts) = (
        air("Crimson Entrance"),
        air("Clover Haven"),
        air("Neon Hearts"),
    );
    let (house, joker, ace) = (air("House of Cards"), air("Joker"), air("Ace Crystal"));
    assert!(crimson.low.x > crimson.low.y && crimson.low.y > crimson.low.z);
    // Humid blue and cyan.
    assert!(clover.low.z > clover.low.x && clover.high.z > clover.high.y);
    // Neon red and pink.
    assert!(hearts.low.x > hearts.low.z && hearts.low.z > hearts.low.y);
    // Purple and magenta.
    assert!(house.low.x > house.low.y && house.low.z > house.low.y);
    // Strange: emerald low, purple high.
    assert!(joker.low.y > joker.low.x && joker.low.y > joker.low.z);
    assert!(joker.high.z > joker.high.y && joker.high.x > joker.high.y);
    // Bright cyan and sky blue, the brightest air.
    assert!(ace.low.y > ace.low.x && ace.low.z > ace.low.x);
    assert!(
        [crimson, clover, hearts, house, joker]
            .iter()
            .all(|a| a.gain < ace.gain)
    );
    assert_eq!(air_of("The King's Crown"), None);
}

#[test]
fn a_halo_rings_the_disk_brightest_on_the_day_side() {
    // The star to the left (-x): the left limb is the day side.
    let shell = clover_shell(Vector3::new(-1.0, 0.0, 0.0));
    let cam = camera(Vector3::new(0.0, 0.0, 20.0), Vector3::zero());
    let mut fb = frame(&cam, &shell);
    let changed = draw_atmospheres(&mut fb, &cam, &[shell]);
    assert!(changed > 0);
    let y = H as i32 / 2;
    let limb = left_limb(&fb, y);
    let right = W as i32 - 1 - limb;
    // Just outside the disk, on both sides; the day side brighter.
    let day = brightness(&fb, limb - 1, y);
    let night = brightness(&fb, right + 1, y);
    assert!(day > 0.2, "{day}");
    assert!(night > 0.0 && night < 0.5 * day, "{night} vs {day}");
    // The color is the planet's air: blue over red.
    let c = fb.get_pixel(limb - 1, y).unwrap();
    assert!(c.z > c.x, "{c:?}");
    // Far from the planet nothing changes.
    assert_eq!(fb.get_pixel(0, 0).unwrap(), Vector3::zero());
    assert_eq!(fb.get_pixel(2, y).unwrap(), Vector3::zero());
}

#[test]
fn the_tint_on_the_disk_grows_towards_the_limb() {
    let shell = clover_shell(Vector3::new(0.0, 0.0, 1.0));
    let cam = camera(Vector3::new(0.0, 0.0, 20.0), Vector3::zero());
    let mut fb = frame(&cam, &shell);
    draw_atmospheres(&mut fb, &cam, &[shell]);
    let y = H as i32 / 2;
    let limb = left_limb(&fb, y);
    let center = brightness(&fb, W as i32 / 2, y);
    let edge = brightness(&fb, limb + 1, y);
    assert!(center > 0.0 && edge > 2.0 * center, "{center} {edge}");
    assert!(center < 0.3, "the disk keeps its own colors: {center}");
}

#[test]
fn what_stands_in_front_hides_the_air() {
    let shell = clover_shell(Vector3::new(-1.0, 0.0, 0.0));
    let cam = camera(Vector3::new(0.0, 0.0, 20.0), Vector3::zero());
    let mut fb = frame(&cam, &shell);
    let y = H as i32 / 2;
    let limb = left_limb(&fb, y);
    // Something 1 u in front of the eye over the halo pixel.
    fb.set_depth(limb - 1, y, 1.0);
    fb.point(limb - 1, y, Vector3::new(0.5, 0.5, 0.5));
    draw_atmospheres(&mut fb, &cam, &[shell]);
    assert_eq!(
        fb.get_pixel(limb - 1, y).unwrap(),
        Vector3::new(0.5, 0.5, 0.5)
    );
}

#[test]
fn from_inside_the_distant_ground_and_horizon_are_hazed() {
    let shell = clover_shell(Vector3::new(0.0, 1.0, 0.0));
    // Low over the day side, looking along the ground.
    let eye = Vector3::new(0.0, 5.6, 0.0);
    let cam = camera(eye, eye + Vector3::new(0.0, -0.3, -1.0));
    let mut fb = frame(&cam, &shell);
    draw_atmospheres(&mut fb, &cam, &[shell]);
    let x = W as i32 / 2;
    // Rows from the bottom (near ground) to the horizon: more haze farther.
    let near = brightness(&fb, x, H as i32 - 2);
    let horizon = (0..H as i32)
        .rev()
        .find(|&y| fb.get_depth(x, y).is_none_or(|z| !z.is_finite()))
        .unwrap();
    let far = brightness(&fb, x, horizon + 1);
    assert!(far > 2.0 * near && far > 0.3, "{near} {far}");
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let c = fb.get_pixel(x, y).unwrap();
            assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
        }
    }
}

#[test]
fn only_the_grand_scale_has_atmospheres() {
    let view = || {
        OrbitView::for_world(
            OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
            8.0,
            Viewport::new(80, 60, 0.1),
        )
        .unwrap()
    };
    let light = Vector3::new(0.0, 1.0, 0.0);
    let project = SolarScene::project3_system(SolarMeshes::load().unwrap(), view(), light).unwrap();
    assert!(
        !project.atmospheres,
        "recordings and oracles keep their frames"
    );
    let grand = SolarScene::grand_system(SolarMeshes::load().unwrap(), view(), light).unwrap();
    assert!(grand.atmospheres);
    let shells = grand.air_shells();
    assert_eq!(shells.len(), 6);
    for (i, s) in (1..).zip(&shells) {
        assert_eq!(s.center, grand.bodies[i].transform.position);
        assert_eq!(s.ground, grand.radius_of(i));
        assert_eq!(s.top, grand.atmosphere_top(i));
        assert!((s.sun.length() - 1.0).abs() < 1e-4);
    }
}

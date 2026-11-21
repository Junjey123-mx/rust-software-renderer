//! Automatic ship lights (Project 3 expansion): navigation lights, a tail
//! strobe and a headlight beam that switch on by themselves when a planet
//! hides the star from the ship and off again in sunlight. Earlier scenes
//! keep the ship without lights.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::scenario_scene_on;
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarScene;
use rust_software_renderer::solar::ship_lights::{shadow, strobe_on};

const STAR: Vector3 = Vector3::new(0.0, 0.0, 0.0);

#[test]
fn shadow_of_a_planet_between_ship_and_star() {
    let planet = [(Vector3::new(10.0, 0.0, 0.0), 1.0)];
    // Behind the planet, on the axis: dark.
    assert_eq!(shadow(Vector3::new(13.0, 0.0, 0.0), STAR, &planet), 1.0);
    // Off the axis by more than 1.15 R, or on the star's side: lit.
    assert_eq!(shadow(Vector3::new(13.0, 2.0, 0.0), STAR, &planet), 0.0);
    assert_eq!(shadow(Vector3::new(7.0, 0.0, 0.0), STAR, &planet), 0.0);
    // The penumbra fades between both.
    let edge = shadow(Vector3::new(13.0, 1.3, 0.0), STAR, &planet);
    assert!(edge > 0.0 && edge < 1.0, "{edge}");
    // Nothing in the way, degenerate input: lit.
    assert_eq!(shadow(Vector3::new(13.0, 0.0, 0.0), STAR, &[]), 0.0);
    assert_eq!(shadow(STAR, STAR, &planet), 0.0);
    assert_eq!(shadow(Vector3::new(f32::NAN, 0.0, 0.0), STAR, &planet), 0.0);
}

#[test]
fn strobe_flashes_once_a_second() {
    let on = (0..1000).filter(|&k| strobe_on(k as f32 * 0.01)).count();
    assert!((100..=140).contains(&on), "{on}");
    assert!(strobe_on(3.05) && !strobe_on(3.5));
    assert!(!strobe_on(f32::NAN));
}

fn scene() -> SolarScene {
    let config = SolarBenchConfig {
        width: 320,
        height: 240,
        ..SolarBenchConfig::default()
    };
    scenario_scene_on("D", &config, true).unwrap()
}

fn frame(s: &SolarScene) -> Vec<u8> {
    let mut fb = Framebuffer::new(320, 240);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb.to_rgba_bytes()
}

/// The ship 1.6 radii from the clover planet, on the side away from the
/// star (`behind`) or facing it, nose along the planet's orbit.
fn park(s: &mut SolarScene, behind: bool) {
    let i = 2;
    let p = s.bodies[i].transform.position;
    let out = (p - s.center).normalized() * if behind { 1.0 } else { -1.0 };
    s.ship.position = p + out * (1.6 * s.radius_of(i));
    s.ship.forward = Vector3::new(0.0, 1.0, 0.0).cross(out).normalized();
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s.ship.renormalize();
}

#[test]
fn on_in_shadow_off_in_sunlight() {
    let mut s = scene();
    assert!(s.ship_lights, "on in the Project 3 system");
    park(&mut s, true);
    assert!(s.ship_shadow() > 0.99);
    let lit = frame(&s);
    s.ship_lights = false;
    let dark = frame(&s);
    let changed = lit
        .chunks(4)
        .zip(dark.chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 30, "{changed} pixels");
    // In sunlight they switch off: the frame is the one without lights.
    let mut s = scene();
    park(&mut s, false);
    assert_eq!(s.ship_shadow(), 0.0);
    let on = frame(&s);
    s.ship_lights = false;
    assert_eq!(on, frame(&s));
}

#[test]
fn earlier_scenes_have_no_lights() {
    let config = SolarBenchConfig {
        width: 80,
        height: 60,
        ..SolarBenchConfig::default()
    };
    assert!(!scenario_scene_on("D", &config, false).unwrap().ship_lights);
}

/// Pixels whose color differs between two frames of the same size.
fn changed(a: &[u8], b: &[u8]) -> usize {
    a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count()
}

#[test]
fn headlight_lights_the_dark_ground_around_the_ship() {
    // E hovers over Crimson Entrance's night side, in full shadow.
    let config = SolarBenchConfig {
        width: 320,
        height: 240,
        ..SolarBenchConfig::default()
    };
    let on = scenario_scene_on("E", &config, true).unwrap();
    assert_eq!(on.ship_shadow(), 1.0);
    let mut off = on.clone();
    off.ship_lights = false;
    let (lit, dark) = (frame(&on), frame(&off));
    // The glows alone change a few hundred pixels at this size; the light
    // on the ground thousands, and only ever brighter.
    let n = changed(&lit, &dark);
    assert!(n > 2500, "{n}");
    let brighter = lit
        .chunks(4)
        .zip(dark.chunks(4))
        .filter(|(p, q)| p != q)
        .all(|(p, q)| (0..3).all(|k| p[k] >= q[k]));
    assert!(brighter);
}

#[test]
fn light_surfaces_stays_within_its_reach() {
    use rust_software_renderer::solar::ship_lights::{LIGHT_REACH, light_surfaces};
    let config = SolarBenchConfig {
        width: 320,
        height: 240,
        ..SolarBenchConfig::default()
    };
    let mut s = scenario_scene_on("E", &config, true).unwrap();
    s.ship_lights = false;
    let mut fb = Framebuffer::new(320, 240);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    let camera = s.follow.camera(&s.ship);
    let before = fb.to_rgba_bytes();
    // Off: nothing. Pushed beyond its reach: nothing either.
    assert_eq!(light_surfaces(&mut fb, &camera, &s.ship, 0.0), 0);
    let mut far = s.ship;
    far.position += far.up * (3.0 * LIGHT_REACH * far.radius());
    assert_eq!(light_surfaces(&mut fb, &camera, &far, 1.0), 0);
    assert_eq!(fb.to_rgba_bytes(), before);
    // On: thousands of pixels of the ground.
    assert!(light_surfaces(&mut fb, &camera, &s.ship, 1.0) > 2500);
}

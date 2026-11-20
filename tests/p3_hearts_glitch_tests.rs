//! Project 3 Hearts glitch: latitude bands picked by a hash of the
//! quantized clock shift the grid and split its red channel. Deterministic,
//! frozen while paused, off at 0, refused above 1, and never in the depth.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::input::{Look, SolarAction};
use rust_software_renderer::solar::planet::hearts::{NeonGridParams, glitch_offset};
use rust_software_renderer::solar::planet::{HeightField, PlanetMaterial};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::viewer::{apply_toggles, step};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 160;
const H: u32 = 120;

/// The project 3 system with the Hearts material on body 1, the ship 2.5
/// radii in front of it.
fn scene(p: NeonGridParams) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap();
    s.bodies[1].surface = Some(PlanetMaterial::Hearts(p));
    s.set_relief(1, HeightField::Hearts(p.field())).unwrap();
    s.clock.time = 2.0;
    s.update();
    let c = s.bodies[1].transform.position;
    s.ship.position = c - Vector3::new(0.0, 0.0, 1.6 * s.radius_of(1));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

fn render(s: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb
}

fn strong() -> NeonGridParams {
    NeonGridParams {
        glitch: 1.0,
        glitch_rate: 1.0,
        ..NeonGridParams::default()
    }
}

#[test]
fn deterministic_at_fixed_time() {
    let p = NeonGridParams::default();
    let mut active = 0;
    for i in 0..400 {
        let (lat, t) = ((i as f32 * 0.37).sin() * 1.5, i as f32 * 0.05);
        let a = glitch_offset(lat, t, &p);
        assert_eq!(a, glitch_offset(lat, t, &p));
        active += (a != (0.0, 0.0)) as usize;
    }
    // About `glitch_rate` of the bands, not all of them, not none.
    assert!((5..120).contains(&active), "{active}");
    assert_eq!(
        render(&scene(p)).to_rgba_bytes(),
        render(&scene(p)).to_rgba_bytes()
    );
}

#[test]
fn paused_glitch_frozen() {
    let mut s = scene(strong());
    // The ship holds still (it starts at cruise speed under gravity).
    s.gravity = false;
    s.ship.velocity = Vector3::zero();
    apply_toggles(&mut s, &[SolarAction::TogglePause]);
    let before = render(&s).to_rgba_bytes();
    for _ in 0..20 {
        step(&mut s, &[], &[], Look::default(), 1.0 / 30.0);
    }
    // The ship did not move (no input), the clock did not run.
    assert_eq!(s.clock.time, 2.0);
    assert_eq!(render(&s).to_rgba_bytes(), before);
}

#[test]
fn glitch_zero_is_identity() {
    let off = NeonGridParams {
        glitch: 0.0,
        glitch_rate: 1.0,
        ..NeonGridParams::default()
    };
    for i in 0..400 {
        let (lat, t) = ((i as f32 * 0.37).sin() * 1.5, i as f32 * 0.05);
        assert_eq!(glitch_offset(lat, t, &off), (0.0, 0.0));
    }
    // With the glitch off, two instants of the same glitch step and of
    // another one only differ by the pulses: a fully glitched frame differs
    // from the clean one, the clean one is reproducible.
    let clean = render(&scene(off)).to_rgba_bytes();
    assert_eq!(clean, render(&scene(off)).to_rgba_bytes());
    assert_ne!(clean, render(&scene(strong())).to_rgba_bytes());
}

#[test]
fn depth_untouched() {
    let clean = render(&scene(NeonGridParams {
        glitch: 0.0,
        ..NeonGridParams::default()
    }));
    let glitched = render(&scene(strong()));
    let mut colors = 0;
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            assert_eq!(
                clean.get_depth(x, y),
                glitched.get_depth(x, y),
                "({x}, {y})"
            );
            colors += (clean.get_pixel(x, y) != glitched.get_pixel(x, y)) as usize;
        }
    }
    assert!(colors > 50, "the glitch shows: {colors} pixels");
}

#[test]
fn glitch_above_one_refused() {
    let mut p = NeonGridParams::default();
    assert!(p.set("glitch", 1.5).is_err());
    assert!(p.set("glitch", -0.1).is_err());
    assert!(p.set("glitch", 1.0).is_ok());
    assert_eq!(p.glitch, 1.0);
}

/// `P3_DUMP_DIR=<dir>` writes the Hearts material at 640x480, clean and
/// fully glitched, as PPM files to look at.
#[test]
fn hearts_frame_dump() {
    let Some(dir) = std::env::var_os("P3_DUMP_DIR") else {
        return;
    };
    for (name, p) in [("clean", NeonGridParams::default()), ("glitch", strong())] {
        let s = scene(p);
        let mut fb = Framebuffer::new(640, 480);
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
        let path = std::path::Path::new(&dir).join(format!("hearts_{name}.ppm"));
        std::fs::write(path, [b"P6 640 480 255\n".as_slice(), &rgb].concat()).unwrap();
    }
}

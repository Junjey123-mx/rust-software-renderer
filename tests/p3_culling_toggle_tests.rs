//! Project 3 culling comparison: with the view test off (`C`,
//! `--no-culling`) every object is drawn and clipped by the rasterizer, so
//! the image is identical and only the work grows.

use raylib::consts::KeyboardKey;
use rust_software_renderer::app::parse_args;
use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{SCENARIOS, scenario_scene};
use rust_software_renderer::celestial::cli::{extract, validate};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::input::{PRESSED, SolarAction};
use rust_software_renderer::solar::viewer::apply_toggles;
use rust_software_renderer::solar::{FrameStats, SolarScene};

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Fingerprint and counters of scenario `id` (800x600, t = 6 s).
fn frame(id: &str, culling: bool) -> (u64, FrameStats) {
    let config = SolarBenchConfig::default();
    let mut scene: SolarScene = scenario_scene(id, &config).unwrap();
    scene.culling = culling;
    let mut fb = Framebuffer::new(config.width, config.height);
    let options = ShadedOptions {
        threads: 4,
        ..ShadedOptions::default()
    };
    scene
        .render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    (fnv1a64(&fb.to_rgba_bytes()), scene.last_stats().clone())
}

#[test]
fn same_fingerprint_with_and_without_culling() {
    for (id, _) in SCENARIOS {
        let (on, _) = frame(id, true);
        let (off, stats) = frame(id, false);
        assert_eq!(on, off, "scenario {id}: culling removed something visible");
        assert_eq!(stats.objects_culled, 0, "scenario {id}");
    }
}

#[test]
fn fewer_triangles_submitted_when_enabled_in_g() {
    let (_, on) = frame("G", true);
    let (_, off) = frame("G", false);
    assert!(on.objects_culled > 0);
    assert!(
        on.triangles_submitted < off.triangles_submitted,
        "{} vs {}",
        on.triangles_submitted,
        off.triangles_submitted
    );
    assert!(on.draws < off.draws);
}

#[test]
fn c_toggles_culling_and_the_flag_starts_without_it() {
    assert!(PRESSED.contains(&(KeyboardKey::KEY_C, SolarAction::ToggleCulling)));
    let mut scene = scenario_scene("D", &SolarBenchConfig::default()).unwrap();
    assert!(scene.culling, "on by default");
    let messages = apply_toggles(&mut scene, &[SolarAction::ToggleCulling]);
    assert!(!scene.culling);
    assert_eq!(messages, ["Frustum culling off"]);

    let parse = |line: &str| {
        let (lab6, rest) = extract(line.split_whitespace().map(str::to_string).collect())?;
        let cli = parse_args(rest)?;
        validate(&cli, &lab6)?;
        Ok::<_, String>(lab6)
    };
    assert!(parse("--scene solar --no-culling").unwrap().no_culling);
    assert!(!parse("--scene solar").unwrap().no_culling);
    assert!(parse("--scene crown --no-culling").is_err());
}

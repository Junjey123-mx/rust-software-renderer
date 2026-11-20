//! Project 3 frame oracle: FNV-1a fingerprints of the whole solar frame for
//! the benchmark scenarios A-J (800x600, t = 6 s, star seed 42, scanline).
//!
//! `LEGACY_A_J` freezes the pre-project solar system as it was before any
//! Project 3 change and is never rewritten: legitimate image changes of
//! Project 3 get their own tables. To print the current fingerprints run
//! `P3_PRINT_BASELINE=1 cargo test --test p3_frame_oracle_tests -- --nocapture`.
//!
//! `P3_SCALED_A_J` fingerprints the same scenarios on the Project 3 system
//! at the project scale (`scenario_scene_on(.., true)`): B, C and D were
//! captured in C23, the rest in C24 once the scenarios follow the layout.

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{SCENARIOS, scenario_scene, scenario_scene_on};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn scenario_hash(id: &str, threads: usize) -> u64 {
    let config = SolarBenchConfig {
        threads,
        ..SolarBenchConfig::default()
    };
    let scene = scenario_scene(id, &config).unwrap();
    let mut fb = Framebuffer::new(config.width, config.height);
    let options = ShadedOptions {
        threads,
        ..ShadedOptions::default()
    };
    scene
        .render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    fnv1a64(&fb.to_rgba_bytes())
}

/// Captured on `ea3b4a8` (pre-project solar system, before Project 3).
const LEGACY_A_J: [(&str, u64); 10] = [
    ("A", 0x43601441c2817a2c),
    ("B", 0x6004661c1e5e8625),
    ("C", 0x589f818dc4ab6f8e),
    ("D", 0x186271a698e69c4b),
    ("E", 0x7d10848cf33e751e),
    ("F", 0x535ba2f24960e47b),
    ("G", 0x7f9c2a88179af11b),
    ("H", 0x186271a698e69c4b),
    ("I", 0x32a8775832c50538),
    ("J", 0xb8631e10f929aafc),
];

fn actual(threads: usize) -> Vec<(&'static str, u64)> {
    SCENARIOS
        .iter()
        .map(|&(id, _)| (id, scenario_hash(id, threads)))
        .collect()
}

#[test]
fn legacy_a_j_match() {
    let got = actual(4);
    if std::env::var_os("P3_PRINT_BASELINE").is_some() {
        for (id, h) in &got {
            println!("    (\"{id}\", {h:#018x}),");
        }
    }
    assert_eq!(got, LEGACY_A_J);
}

#[test]
fn threads_do_not_change_pixels() {
    for id in ["D", "J"] {
        assert_eq!(scenario_hash(id, 1), scenario_hash(id, 20), "{id}");
    }
}

#[test]
fn two_runs_are_identical() {
    for id in ["C", "E"] {
        assert_eq!(scenario_hash(id, 4), scenario_hash(id, 4), "{id}");
    }
}

#[test]
fn a_single_flipped_bit_is_detected() {
    let (id, h) = LEGACY_A_J[3];
    assert_ne!(scenario_hash(id, 4), h ^ 1);
}

fn scaled_hash(id: &str) -> u64 {
    let config = SolarBenchConfig {
        threads: 4,
        ..SolarBenchConfig::default()
    };
    let scene = scenario_scene_on(id, &config, true).unwrap();
    let mut fb = Framebuffer::new(config.width, config.height);
    let options = ShadedOptions {
        threads: 4,
        ..ShadedOptions::default()
    };
    scene
        .render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    let rgba = fb.to_rgba_bytes();
    // `P3_DUMP_DIR=<dir>` writes the frame as a PPM, to look at it.
    if let Some(dir) = std::env::var_os("P3_DUMP_DIR") {
        let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        let header = format!("P6 {} {} 255\n", config.width, config.height);
        let path = std::path::Path::new(&dir).join(format!("p3_scaled_{id}.ppm"));
        std::fs::write(path, [header.as_bytes(), &rgb].concat()).unwrap();
    }
    fnv1a64(&rgba)
}

/// Project 3 system at the project scale: B, C and D captured on C23, the
/// others on C24.
const P3_SCALED_A_J: [(&str, u64); 10] = [
    ("A", 0xc45cefa3d2a9ddcc),
    ("B", 0xe59e2a67ca2766b7),
    ("C", 0x1c464b6c8d4dc9d1),
    ("D", 0xee1ab5d5ddd77b56),
    ("E", 0xf94f0363bdf79a8e),
    ("F", 0xf9fde6c318a39b8d),
    ("G", 0xd04d0613d738e4a5),
    ("H", 0xee1ab5d5ddd77b56),
    ("I", 0x7c27712dab665184),
    ("J", 0x927e6b592e39779f),
];

#[test]
fn p3_scaled_a_j_match() {
    let got: Vec<(&str, u64)> = SCENARIOS
        .iter()
        .map(|&(id, _)| (id, scaled_hash(id)))
        .collect();
    if std::env::var_os("P3_PRINT_BASELINE").is_some() {
        for (id, h) in &got {
            println!("    (\"{id}\", {h:#018x}),");
        }
    }
    assert_eq!(got, P3_SCALED_A_J);
    // The scaled frames are not the legacy ones (A, the ship alone, is).
    for ((id, h), (_, legacy)) in got.iter().zip(LEGACY_A_J).skip(1) {
        assert_ne!(*h, legacy, "{id}");
    }
}

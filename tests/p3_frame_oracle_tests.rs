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
//! captured in C23, the rest in C24 once the scenarios follow the layout,
//! all recaptured in C49 with the clover planet and in C61 with the hearts
//! planet; C, D, H and I recaptured on 2026-11-20 with the Joker, the
//! fifth planet (they see the whole system), and every Project 3 frame
//! with a sky (all but I) recaptured the same day with the purple-blue sky
//! cycle (`nebula::blue_share`, t = 6); A, C, D, E, H, I and J once more
//! with the comets (`solar::comets`), which cross those views at t = 6. `P3_CLOVER_K` adds the clover planet scenarios K, K2 (C49) and
//! K3 (C52); `P3_HEARTS_M` the hearts planet scenarios M and M2 (C61).

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{
    CLOVER_SCENARIOS, HEARTS_SCENARIOS, SCENARIOS, scenario_scene, scenario_scene_on,
};
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
/// others on C24. Recaptured on C49, which adds the clover planet between
/// Crimson Entrance and House of Cards (House moves from 36 to 40, so the
/// layout, the world radius and the spawn grow): A, C, D, F, G, H and I
/// change; B, E and J (Crimson Entrance alone or close up) do not.
/// Recaptured on C61, which adds the hearts planet at orbit 29.5 between
/// the clover and House of Cards: C (the orbit view shows it), J (it rises
/// behind Crimson Entrance) and E (two pixels of its heart disc between
/// the ship and the horizon) change; the layout keeps House at 40, so the
/// rest do not. Recaptured on C65, which adds the neon ships over Hearts:
/// C and J change (ships in view), the rest do not. Recaptured on C68,
/// which adds the trails: C and J change again.
const P3_SCALED_A_J: [(&str, u64); 10] = [
    ("A", 0x8a73f9d1b1215430),
    ("B", 0xd1a777942fd18077),
    ("C", 0x563ce30f0c01581),
    ("D", 0x4cfe1f7db38213e),
    ("E", 0xc12abe131a31ff3f),
    ("F", 0xe1a437e48dd1ea99),
    ("G", 0x43325ccd345dcfdf),
    ("H", 0x4cfe1f7db38213e),
    ("I", 0xdc6e9cf26dee134d),
    ("J", 0x3eb104ff2a893dc8),
];

/// Clover planet scenarios of the scaled benchmark, captured on C49. C52
/// adds the waterfalls (K changes, K2 is too far to show them) and K3, a
/// mountain and its waterfall up close.
const P3_CLOVER_K: [(&str, u64); 3] = [
    ("K", 0xd7979c26db1c14e8),
    ("K2", 0xc30d6353c1421276),
    ("K3", 0xa9dcbef19eec66e2),
];

/// Hearts planet scenarios of the scaled benchmark, captured on C61: the
/// pyramid and the city up close (M) and the planet 12 radii away (M2).
/// Recaptured on C65: both show the neon ships now. C67 adds the
/// lightcycles: M changes, M2 (planet under 60 px) does not. C68 adds the
/// trails: both change (the ships' ribbons show from afar too).
const P3_HEARTS_M: [(&str, u64); 2] = [("M", 0xb912607e99d028d6), ("M2", 0x720ddd197330d370)];

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

#[test]
fn p3_clover_k_match() {
    let got: Vec<(&str, u64)> = CLOVER_SCENARIOS
        .iter()
        .map(|&(id, _)| (id, scaled_hash(id)))
        .collect();
    if std::env::var_os("P3_PRINT_BASELINE").is_some() {
        for (id, h) in &got {
            println!("    (\"{id}\", {h:#018x}),");
        }
    }
    assert_eq!(got, P3_CLOVER_K);
    assert_ne!(got[0].1, got[1].1);
}

#[test]
fn p3_hearts_m_match() {
    let got: Vec<(&str, u64)> = HEARTS_SCENARIOS
        .iter()
        .map(|&(id, _)| (id, scaled_hash(id)))
        .collect();
    if std::env::var_os("P3_PRINT_BASELINE").is_some() {
        for (id, h) in &got {
            println!("    (\"{id}\", {h:#018x}),");
        }
    }
    assert_eq!(got, P3_HEARTS_M);
    assert_ne!(got[0].1, got[1].1);
}

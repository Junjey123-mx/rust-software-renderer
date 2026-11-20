//! Project 3 relief level of detail under the benchmark: each level draws
//! the same frame every time (fresh scenes, same counters), and the
//! automatic level never submits more triangles than level 5 forced.

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{LOD_SCENARIOS, scenario_scene_on};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarScene;

fn config() -> SolarBenchConfig {
    SolarBenchConfig {
        width: 200,
        height: 150,
        ..SolarBenchConfig::default()
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Fingerprint, relief level and relief triangles of one frame.
fn frame(s: &SolarScene) -> (u64, Option<u8>, usize) {
    let c = config();
    let mut fb = Framebuffer::new(c.width, c.height);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    let body = s.last_stats().per_body[1];
    (fnv1a64(&fb.to_rgba_bytes()), body.lod, body.triangles)
}

#[test]
fn each_level_fingerprint_stable() {
    for level in 2..=5u8 {
        let run = || {
            let mut s = scenario_scene_on("L2", &config(), true).unwrap();
            s.lod_force = Some(level);
            frame(&s)
        };
        let (a, b) = (run(), run());
        assert_eq!(a, b, "level {level}: not deterministic");
        assert_eq!(a.1, Some(level));
        assert_eq!(a.2, 20 * 4usize.pow(level as u32));
    }
    // Different levels are different images (the level really changes).
    let hash = |level| {
        let mut s = scenario_scene_on("L2", &config(), true).unwrap();
        s.lod_force = Some(level);
        frame(&s).0
    };
    assert_ne!(hash(2), hash(5));
}

#[test]
fn auto_lod_not_more_triangles_than_forced() {
    for pair in LOD_SCENARIOS.chunks(2) {
        let (auto, forced) = (pair[0].0, pair[1].0);
        let a = frame(&scenario_scene_on(auto, &config(), true).unwrap());
        let f = frame(&scenario_scene_on(forced, &config(), true).unwrap());
        assert_eq!(f.1, Some(5), "{forced}");
        assert!(a.2 <= f.2, "{auto}: {} > {}", a.2, f.2);
    }
    // At the benchmark size (800x600) the three cases use levels 2, 4 and
    // 4: the grazing camera sees about 283 px of radius, under the 360 px
    // of level 5 (reached from about 770 px of frame height).
    let levels: Vec<Option<u8>> = ["L1", "L2", "L3"]
        .iter()
        .map(|id| {
            let s = scenario_scene_on(id, &SolarBenchConfig::default(), true).unwrap();
            let c = SolarBenchConfig::default();
            let mut fb = Framebuffer::new(c.width, c.height);
            s.render_frame(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions::default(),
            )
            .unwrap();
            s.last_stats().per_body[1].lod
        })
        .collect();
    assert_eq!(levels, [Some(2), Some(4), Some(4)]);
    // Names fit in one CSV field.
    assert!(LOD_SCENARIOS.iter().all(|(_, name)| !name.contains(',')));
    // The relief cases exist on the scaled system only.
    assert!(scenario_scene_on("L1", &config(), false).is_err());
}

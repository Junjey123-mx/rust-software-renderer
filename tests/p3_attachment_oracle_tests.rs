//! Project 3 attachment migration: the crown and the card ring are drawn
//! from `SolarScene::attachments` (host by id, list order = draw order)
//! with the same pixels as before.

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{SCENARIOS, scenario_scene};
use rust_software_renderer::pipeline::ShadedOptions;
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarScene;
use rust_software_renderer::solar::attach::AttachmentKind;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::system::PlanetId;

/// The pre-project fingerprints (`tests/p3_frame_oracle_tests.rs`).
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

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

fn hash(s: &SolarScene) -> u64 {
    let mut fb = Framebuffer::new(800, 600);
    let options = ShadedOptions {
        threads: 4,
        ..ShadedOptions::default()
    };
    s.render_frame(&mut fb, TriangleMode::FilledScanline, options)
        .unwrap();
    fnv1a64(&fb.to_rgba_bytes())
}

fn kinds(s: &SolarScene) -> Vec<&'static str> {
    s.effect_draws(800, 600, 1)
        .iter()
        .map(|d| match d.shader {
            EffectShader::Crown(_) => "crown",
            EffectShader::Card(_) => "card",
            EffectShader::Flame(_) => "flame",
            EffectShader::Prop(_) => "prop",
        })
        .collect()
}

#[test]
fn crown_and_ring_identical_a_j() {
    for ((id, _), (legacy_id, legacy)) in SCENARIOS.iter().zip(LEGACY_A_J) {
        assert_eq!(*id, legacy_id);
        let s = scenario_scene(id, &SolarBenchConfig::default()).unwrap();
        assert_eq!(hash(&s), legacy, "scenario {id}");
    }
}

#[test]
fn draws_follow_the_attachment_list() {
    let s = scenario_scene("D", &SolarBenchConfig::default()).unwrap();
    let draws = kinds(&s);
    // Crown (gold, jewels), then the visible cards, then the three flames.
    assert_eq!(&draws[..2], ["crown", "crown"]);
    assert!(draws[2..draws.len() - 3].iter().all(|k| *k == "card"));
    assert_eq!(&draws[draws.len() - 3..], ["flame"; 3]);

    // Without its attachment the ring is not drawn, though `cards` is set.
    let mut bare = s.clone();
    bare.attachments
        .retain(|a| a.kind != AttachmentKind::CardRing);
    assert!(bare.cards.is_some());
    assert!(!kinds(&bare).contains(&"card"));
    assert_eq!(bare.card_host(), None);
}

#[test]
fn the_host_is_found_by_id() {
    let mut s = scenario_scene("D", &SolarBenchConfig::default()).unwrap();
    assert_eq!(s.card_host(), s.body_index(PlanetId("house")));
    let ring = s
        .attachments
        .iter_mut()
        .find(|a| a.kind == AttachmentKind::CardRing)
        .unwrap();
    ring.host = PlanetId("entrance");
    assert_eq!(s.card_host(), s.body_index(PlanetId("entrance")));
    // An unknown host draws nothing instead of failing.
    s.attachments[1].host = PlanetId("moon");
    assert_eq!(s.card_host(), None);
    assert!(!kinds(&s).contains(&"card"));
}

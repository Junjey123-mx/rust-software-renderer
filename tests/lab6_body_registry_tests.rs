//! Body registry, parameter tables and the shared-sphere scene.

use rust_software_renderer::celestial::{Body, CelestialMaterial, CelestialScene, DebugView};
use rust_software_renderer::pipeline::{ShadedOptions, ViewPreset};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;

#[test]
fn body_names_round_trip_and_cycle() {
    for body in Body::ALL {
        assert_eq!(body.name().parse::<Body>().unwrap(), body);
        assert_eq!(body.next().next().next(), body);
        assert_ne!(body.next(), body);
    }
    assert_eq!(Body::default(), Body::KingsCrown);
    assert_eq!(Body::HouseOfCards.display_name(), "House of Cards");
    for view in DebugView::ALL {
        assert_eq!(view.name().parse::<DebugView>().unwrap(), view);
    }
    assert_eq!(DebugView::Parallax.next(), DebugView::None);
}

#[test]
fn unknown_body_error_names_the_input() {
    let err = "comet".parse::<Body>().unwrap_err();
    assert!(err.contains("comet") && err.contains("crown"));
    assert!("glow".parse::<DebugView>().unwrap_err().contains("glow"));
}

#[test]
fn param_specs_are_unique_in_range_and_match_defaults() {
    for body in Body::ALL {
        let material = CelestialMaterial::for_body(body);
        let specs = material.specs();
        assert!(!specs.is_empty());
        for (i, spec) in specs.iter().enumerate() {
            assert!(
                specs[i + 1..].iter().all(|s| s.name != spec.name),
                "{body}: duplicated {}",
                spec.name
            );
            assert!(
                spec.min <= spec.default && spec.default <= spec.max,
                "{}",
                spec.name
            );
            assert!(!spec.help.is_empty());
            assert_eq!(material.get(spec.name), Some(spec.default), "{}", spec.name);
        }
        assert_eq!(material.body(), body);
    }
}

#[test]
fn set_validates_name_and_range() {
    let mut material = CelestialMaterial::for_body(Body::KingsCrown);
    material.set("brightness", 1.5).unwrap();
    assert_eq!(material.get("brightness"), Some(1.5));
    let unknown = material.set("sparkle", 1.0).unwrap_err();
    assert!(unknown.contains("sparkle") && unknown.contains("brightness"));
    assert!(
        material
            .set("brightness", 99.0)
            .unwrap_err()
            .contains("outside")
    );
    assert!(material.set("brightness", f32::NAN).is_err());
    assert_eq!(material.get("brightness"), Some(1.5));
}

#[test]
fn every_body_uses_the_same_sphere_positions() {
    let scenes: Vec<CelestialScene> = Body::ALL
        .into_iter()
        .map(|b| CelestialScene::new(b, ViewPreset::Oblique, 200, 150).unwrap())
        .collect();
    let positions = |s: &CelestialScene| s.vertices.iter().map(|v| v.position).collect::<Vec<_>>();
    // 576 expanded corners = 192 triangles x 3 from the 98 unique vertices.
    assert_eq!(scenes[0].vertices.len(), 576);
    assert_eq!(positions(&scenes[0]), positions(&scenes[1]));
    assert_eq!(positions(&scenes[0]), positions(&scenes[2]));
    let mut switched =
        CelestialScene::new(Body::KingsCrown, ViewPreset::Oblique, 200, 150).unwrap();
    switched.time = 3.0;
    switched.set_body(Body::HouseOfCards);
    assert_eq!(switched.body(), Body::HouseOfCards);
    assert_eq!(switched.time, 3.0);
    assert_eq!(positions(&switched), positions(&scenes[0]));
}

#[test]
fn every_body_renders_finite_colors() {
    for body in Body::ALL {
        let scene = CelestialScene::new(body, ViewPreset::Oblique, 200, 150).unwrap();
        let mut fb = Framebuffer::new(200, 150);
        let stats = scene
            .render(
                &mut fb,
                TriangleMode::FilledScanline,
                ShadedOptions::default(),
            )
            .unwrap();
        assert!(fb.count_non_background() > 0, "{body}");
        assert_eq!(stats.fragments_shaded, fb.count_non_background(), "{body}");
        for y in 0..150 {
            for x in 0..200 {
                let c = fb.get_pixel(x, y).unwrap();
                assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
            }
        }
    }
}

//! Project 3 surface props: the same seed gives the same poses, another
//! seed moves them, none stands under water, the spacing holds and an
//! impossible rule gives no props without a panic.

use rust_software_renderer::solar::attach::props::{PropKind, PropPose, ScatterRule, scatter};
use rust_software_renderer::solar::planet::{CloverField, HeightField};

fn field() -> HeightField {
    HeightField::Clover(CloverField::new(7))
}

/// Share of `a` with no pose of `b` within 0.002 rad.
fn moved_share(a: &[PropPose], b: &[PropPose]) -> f32 {
    let moved = a
        .iter()
        .filter(|p| b.iter().all(|q| p.dir.dot(q.dir) < 0.002f32.cos()))
        .count();
    moved as f32 / a.len() as f32
}

#[test]
fn same_seed_same_poses() {
    let a = scatter(&field(), &ScatterRule::TREES, 11);
    let b = scatter(&field(), &ScatterRule::TREES, 11);
    assert_eq!(a, b);
    assert!(a.len() >= 100, "hundreds of trees: {}", a.len());
    assert!(a.iter().all(|p| p.kind == PropKind::Tree));
    let rocks = scatter(&field(), &ScatterRule::ROCKS, 11);
    assert!(!rocks.is_empty() && rocks.iter().all(|p| p.kind == PropKind::Rock));
}

#[test]
fn different_seed_moves_props() {
    let a = scatter(&field(), &ScatterRule::TREES, 11);
    let b = scatter(&field(), &ScatterRule::TREES, 12);
    assert_ne!(a, b);
    // No hard-coded positions: more than 90 % of the poses move.
    assert!(moved_share(&a, &b) > 0.9, "{}", moved_share(&a, &b));
    assert!(moved_share(&b, &a) > 0.9);
}

#[test]
fn none_under_water() {
    for seed in [1, 2, 3] {
        for rule in [ScatterRule::TREES, ScatterRule::ROCKS] {
            for p in scatter(&field(), &rule, seed) {
                assert!(field().height(p.dir) > field().sea_level().unwrap());
                assert!((p.dir.length() - 1.0).abs() < 1e-4);
                assert!((p.up.length() - 1.0).abs() < 1e-4 && p.up.dot(p.dir) > 0.5);
                assert!((0.8..1.2).contains(&p.scale) && p.yaw.is_finite());
            }
        }
    }
}

#[test]
fn min_spacing_respected() {
    for rule in [ScatterRule::TREES, ScatterRule::ROCKS] {
        let poses = scatter(&field(), &rule, 5);
        for (i, a) in poses.iter().enumerate() {
            for b in &poses[i + 1..] {
                let angle = a.dir.dot(b.dir).clamp(-1.0, 1.0).acos();
                assert!(angle >= rule.min_spacing * 0.999, "{angle}");
            }
        }
    }
}

#[test]
fn impossible_rule_gives_no_props() {
    let flat = ScatterRule {
        max_slope: 0.0,
        ..ScatterRule::TREES
    };
    assert!(scatter(&field(), &flat, 3).is_empty());
    // The ideal sphere has no sea: all of it counts as land.
    assert!(!scatter(&HeightField::Flat, &ScatterRule::TREES, 3).is_empty());
    let no_candidates = ScatterRule {
        candidates: 0,
        ..ScatterRule::TREES
    };
    assert!(scatter(&field(), &no_candidates, 3).len() <= 1);
}

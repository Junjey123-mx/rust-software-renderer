//! Project 3 Hearts city: hundreds of neon buildings scattered on the flat
//! terraces by seed, outside the pyramid's footprint, baked into a few
//! sector meshes per part.

use raylib::math::Vector3;
use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::attach::props::{
    PropKind, PropPart, SECTOR_COUNT, ScatterRule, bake, scatter,
};
use rust_software_renderer::solar::planet::{HeartsField, HeightField};

fn field() -> HeightField {
    HeightField::Hearts(HeartsField::new(11))
}

fn city(seed: u32) -> Vec<rust_software_renderer::solar::attach::props::PropPose> {
    scatter(&field(), &ScatterRule::BUILDINGS, seed)
}

#[test]
fn same_seed_same_district() {
    let a = city(11);
    assert_eq!(a, city(11));
    assert!(
        (100..1000).contains(&a.len()),
        "hundreds of buildings: {}",
        a.len()
    );
    assert!(a.iter().all(|p| p.kind == PropKind::Building));
    let b = city(12);
    let moved = b
        .iter()
        .filter(|p| a.iter().all(|q| p.dir.dot(q.dir) < 0.9999))
        .count();
    assert!(moved * 10 > b.len() * 9, "{moved} of {}", b.len());
}

#[test]
fn only_on_plateaus() {
    let f = field();
    for p in city(11) {
        assert!(f.is_plateau(p.dir), "{:?}", p.dir);
    }
    // No terraces, no city: the clover and the ideal sphere have none.
    assert!(scatter(&HeightField::Flat, &ScatterRule::BUILDINGS, 11).is_empty());
}

#[test]
fn pyramid_footprint_empty() {
    let all = city(11);
    let center = all[all.len() / 2].dir;
    let radius = 0.2f32;
    let near = |p: &&rust_software_renderer::solar::attach::props::PropPose| {
        p.dir.dot(center) > radius.cos()
    };
    assert!(
        all.iter().filter(near).count() > 0,
        "the cone has buildings without the exclusion"
    );
    let rule = ScatterRule {
        exclude: Some((center, radius)),
        ..ScatterRule::BUILDINGS
    };
    let kept = scatter(&field(), &rule, 11);
    assert_eq!(kept.iter().filter(near).count(), 0);
    assert!(kept.len() > all.len() / 2, "the rest of the city stays");
}

#[test]
fn draws_are_sectors() {
    let poses = city(11);
    let layers = bake(&poses, &field(), SPHERE_RADIUS, 0.06 * SPHERE_RADIUS, 11);
    let parts: Vec<PropPart> = layers.layers.iter().map(|l| l.part).collect();
    assert_eq!(parts.len(), 2);
    assert!(parts.contains(&PropPart::BuildingBody) && parts.contains(&PropPart::BuildingEdge));
    assert!(layers.sector_count() <= 2 * SECTOR_COUNT);
    // Hundreds of buildings, a couple of dozen meshes at most.
    for l in &layers.layers {
        let baked: usize = l.sectors.iter().map(|s| s.props).sum();
        assert_eq!(baked, poses.len());
    }
    assert!(PropPart::BuildingEdge.glow() > 0.0 && PropPart::BuildingBody.glow() == 0.0);
}

#[test]
fn exclusion_of_the_whole_planet_leaves_none() {
    let rule = ScatterRule {
        exclude: Some((Vector3::new(0.0, 1.0, 0.0), std::f32::consts::PI)),
        ..ScatterRule::BUILDINGS
    };
    assert!(scatter(&field(), &rule, 11).is_empty());
}

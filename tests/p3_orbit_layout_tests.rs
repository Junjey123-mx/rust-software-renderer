//! Project 3 orbit layout: radii that increase with the spec order and
//! shells (body plus attachments) that never meet, with a fifth slot kept.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::Attachment;
use rust_software_renderer::solar::card_ring::CardRing;
use rust_software_renderer::solar::plasma_loops::PlasmaLoops;
use rust_software_renderer::solar::royal_crown::RoyalCrown;
use rust_software_renderer::solar::system::layout::{
    FIFTH_SLOT_RESERVED_EXTENT, OrbitLayout, WORLD_MARGIN, effective_radius, star_shell,
};
use rust_software_renderer::solar::system::spec::AUTO_ORBIT;
use rust_software_renderer::solar::system::{PlanetId, SpecError, SystemSpec};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const GAP: f32 = 3.0;

/// Four planets of the design sizes (two copies of each pre-project
/// planet), every radius left to the solver, and their effective radii.
fn four_planets() -> (SystemSpec, Vec<f32>, f32) {
    let mut spec = SystemSpec::preproject3();
    let (entrance, house) = (spec.planets[0].clone(), spec.planets[1].clone());
    spec.planets = vec![entrance.clone(), house.clone(), entrance, house];
    for (p, id) in spec.planets.iter_mut().zip(["a", "b", "c", "d"]) {
        p.id = PlanetId(id);
        p.orbit_radius = AUTO_ORBIT;
    }
    let ring = CardRing::diamonds();
    let eff = spec
        .planets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let attachments = if i % 2 == 1 {
                vec![Attachment::CardRing(&ring)]
            } else {
                vec![]
            };
            effective_radius(p, &attachments).unwrap()
        })
        .collect();
    let crown = RoyalCrown::royal();
    let shell = star_shell(
        &spec.star,
        &[Attachment::RoyalCrown(&crown)],
        &PlasmaLoops::crown(),
    )
    .unwrap();
    (spec, eff, shell)
}

#[test]
fn radii_strictly_increase() {
    let (spec, eff, shell) = four_planets();
    let layout = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    assert_eq!(layout.radii.len(), 4);
    assert!(layout.radii.windows(2).all(|w| w[0] < w[1]), "{layout:?}");
    // Automatic radii are the bound rounded up to 0.5.
    assert!(layout.radii.iter().all(|r| (r * 2.0).fract() == 0.0));
    assert!(layout.radii[0] - eff[0] - (shell + GAP) < 0.5);
}

#[test]
fn shells_never_intersect() {
    let (spec, eff, shell) = four_planets();
    let layout = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    let mut edge = shell;
    for (r, e) in layout.radii.iter().zip(&eff) {
        assert!(r - e >= edge + GAP, "{r} - {e} < {edge} + {GAP}");
        edge = r + e;
    }
    assert_eq!(layout.world_radius, edge + WORLD_MARGIN);
}

#[test]
fn explicit_radius_respected() {
    let (mut spec, eff, shell) = four_planets();
    let auto = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    spec.planets[1].orbit_radius = auto.radii[1] + 2.25;
    let layout = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    assert_eq!(layout.radii[1], auto.radii[1] + 2.25);
    assert_eq!(layout.radii[0], auto.radii[0]);
    // The planets after it move out with it.
    assert!(layout.radii[2] > auto.radii[2]);
    // The pre-project radii meet a small gap and are kept as they are.
    let pre = SystemSpec::preproject3();
    let ring = CardRing::diamonds();
    let eff = [
        effective_radius(&pre.planets[0], &[]).unwrap(),
        effective_radius(&pre.planets[1], &[Attachment::CardRing(&ring)]).unwrap(),
    ];
    let layout = OrbitLayout::solve(&pre, &eff, shell, 0.3).unwrap();
    assert_eq!(layout.radii, vec![3.0, 6.2]);
}

#[test]
fn explicit_radius_violation_rejected() {
    let (mut spec, eff, shell) = four_planets();
    let auto = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    // Invading the inner neighbor's shell is an error naming the planet.
    spec.planets[2].orbit_radius = auto.radii[1] + 1.0;
    let err = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap_err();
    match err {
        SpecError::OrbitOverlap {
            planet,
            radius,
            min,
        } => {
            assert_eq!(planet, PlanetId("c"));
            assert_eq!(radius, auto.radii[1] + 1.0);
            assert_eq!(min, auto.radii[1] + eff[1] + GAP + eff[2]);
        }
        other => panic!("{other:?}"),
    }
    // Two planets on the same radius too.
    let (mut spec, ..) = four_planets();
    spec.planets[0].orbit_radius = 20.0;
    spec.planets[1].orbit_radius = 20.0;
    assert!(matches!(
        OrbitLayout::solve(&spec, &eff, shell, GAP),
        Err(SpecError::OrbitOverlap {
            planet: PlanetId("b"),
            ..
        })
    ));
    // The pre-project system does not meet the project gap.
    let pre = SystemSpec::preproject3();
    assert!(OrbitLayout::solve(&pre, &eff[..2], shell, GAP).is_err());
    // Effective radii must match the planets.
    assert!(OrbitLayout::solve(&pre, &eff, shell, GAP).is_err());
}

#[test]
fn fifth_slot_beyond_house() {
    let (spec, eff, shell) = four_planets();
    let layout = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    let last = layout.radii[3] + eff[3];
    assert!(layout.fifth_slot - FIFTH_SLOT_RESERVED_EXTENT >= last + GAP);
    // Not registered: it does not widen the world.
    assert!(layout.world_radius < layout.fifth_slot + FIFTH_SLOT_RESERVED_EXTENT);
}

#[test]
fn radius_is_time_independent() {
    let (mut spec, eff, shell) = four_planets();
    let layout = OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap();
    assert_eq!(OrbitLayout::solve(&spec, &eff, shell, GAP).unwrap(), layout);
    layout.apply(&mut spec);
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        layout.world_radius,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &spec,
    )
    .unwrap();
    for t in [0.0, 7.3, 120.0] {
        s.clock.time = t;
        s.update();
        for (b, r) in s.bodies[1..].iter().zip(&layout.radii) {
            assert!(((b.transform.position - s.center).length() - r).abs() < 1e-3);
        }
    }
    // An automatic radius is not a scene: the layout comes first.
    let (spec, ..) = four_planets();
    assert_eq!(
        spec.validate(),
        Err(SpecError::OutOfRange("planet.orbit_radius", AUTO_ORBIT))
    );
}

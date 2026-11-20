//! Project 3 effective extents: the room each body takes with its
//! attachments, measured on the real geometry.

use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::attach::{Attachment, max_extent};
use rust_software_renderer::solar::card_ring::CardRing;
use rust_software_renderer::solar::plasma_loops::PlasmaLoops;
use rust_software_renderer::solar::royal_crown::RoyalCrown;
use rust_software_renderer::solar::system::SpecError;
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::system::layout::{
    FIFTH_SLOT_RESERVED_EXTENT, effective_radius, star_shell,
};

#[test]
fn crown_shell_matches_measured() {
    let spec = SystemSpec::preproject3();
    let crown = RoyalCrown::royal();
    let loops = PlasmaLoops::crown();
    let r = spec.star.scale * SPHERE_RADIUS;
    // The crown (1.7615 R) reaches past the loops (1.47 R).
    assert!(crown.extent > loops.extent());
    let shell = star_shell(&spec.star, &[Attachment::RoyalCrown(&crown)], &loops).unwrap();
    assert_eq!(shell, r * crown.extent);
    // Without the crown the loops set it.
    assert_eq!(
        star_shell(&spec.star, &[], &loops).unwrap(),
        r * loops.extent()
    );
}

#[test]
fn ring_shell_matches_measured() {
    let spec = SystemSpec::preproject3();
    let house = &spec.planets[1];
    let ring = CardRing::diamonds();
    let eff = effective_radius(house, &[Attachment::CardRing(&ring)]).unwrap();
    assert_eq!(eff, house.radius() * ring.extent());
    // A planet without attachments takes its own radius.
    assert_eq!(
        effective_radius(&spec.planets[0], &[]).unwrap(),
        spec.planets[0].radius()
    );
}

#[test]
fn shells_do_not_overlap_in_preproject3() {
    let spec = SystemSpec::preproject3();
    let crown = RoyalCrown::royal();
    let ring = CardRing::diamonds();
    let star = star_shell(
        &spec.star,
        &[Attachment::RoyalCrown(&crown)],
        &PlasmaLoops::crown(),
    )
    .unwrap();
    let entrance = &spec.planets[0];
    let house = &spec.planets[1];
    let e = effective_radius(entrance, &[]).unwrap();
    let h = effective_radius(house, &[Attachment::CardRing(&ring)]).unwrap();
    // Each shell ends before the next one starts, over the whole orbit.
    assert!(
        star < entrance.orbit_radius - e,
        "{star} vs {}",
        entrance.orbit_radius - e
    );
    assert!(entrance.orbit_radius + e < house.orbit_radius - h);
}

#[test]
fn fifth_slot_reserved() {
    assert_eq!(FIFTH_SLOT_RESERVED_EXTENT, 1.95);
}

#[test]
fn nan_extent_is_an_error() {
    let mut crown = RoyalCrown::royal();
    crown.extent = f32::NAN;
    let crown = [Attachment::RoyalCrown(&crown)];
    assert_eq!(
        max_extent(&crown),
        Err(SpecError::NonFinite("attachment.extent"))
    );
    let spec = SystemSpec::preproject3();
    assert!(effective_radius(&spec.planets[0], &crown).is_err());
    assert!(star_shell(&spec.star, &crown, &PlasmaLoops::crown()).is_err());
    assert_eq!(max_extent(&[]), Ok(0.0));
}

//! Project 3 attachment contract: hosts by id with clean errors,
//! conservative extents equal to the existing geometry, and attachment
//! space that follows the host's spin.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::Transform;
use rust_software_renderer::solar::attach::{
    Attachment, AttachmentKind, AttachmentSpec, HostPose, bind_attachments,
};
use rust_software_renderer::solar::card_ring::CardRing;
use rust_software_renderer::solar::royal_crown::RoyalCrown;
use rust_software_renderer::solar::system::{PlanetId, SpecError};

const HOSTS: [PlanetId; 3] = [PlanetId("crown"), PlanetId("entrance"), PlanetId("house")];

fn spec(kind: AttachmentKind, host: &'static str) -> AttachmentSpec {
    AttachmentSpec {
        kind,
        host: PlanetId(host),
    }
}

#[test]
fn hosts_resolve_to_body_indices_in_order() {
    let specs = [
        spec(AttachmentKind::RoyalCrown, "crown"),
        spec(AttachmentKind::CardRing, "house"),
    ];
    assert_eq!(bind_attachments(&specs, &HOSTS), Ok(vec![0, 2]));
}

#[test]
fn unknown_host_is_clean_error() {
    let specs = [spec(AttachmentKind::CardRing, "moon")];
    assert_eq!(
        bind_attachments(&specs, &HOSTS),
        Err(SpecError::UnknownHost(PlanetId("moon")))
    );
    assert_eq!(
        bind_attachments(&specs, &[]),
        Err(SpecError::UnknownHost(PlanetId("moon")))
    );
}

#[test]
fn duplicate_crown_rejected() {
    let specs = [
        spec(AttachmentKind::RoyalCrown, "crown"),
        spec(AttachmentKind::RoyalCrown, "crown"),
    ];
    assert_eq!(
        bind_attachments(&specs, &HOSTS),
        Err(SpecError::DuplicateId(PlanetId("crown")))
    );
    // The same kind on another host is fine.
    let specs = [
        spec(AttachmentKind::CardRing, "entrance"),
        spec(AttachmentKind::CardRing, "house"),
    ];
    assert!(bind_attachments(&specs, &HOSTS).is_ok());
}

#[test]
fn extent_matches_existing_methods() {
    let crown = RoyalCrown::royal();
    let ring = CardRing::diamonds();
    assert_eq!(Attachment::RoyalCrown(&crown).extent(), crown.extent);
    assert_eq!(Attachment::CardRing(&ring).extent(), ring.extent());
    // Measured on the code (document B said 1.751 R for the crown; the
    // code gives 1.7615 R and document B was corrected in C14).
    assert!((crown.extent - 1.7615).abs() < 1e-3, "{}", crown.extent);
    assert!((ring.extent() - 2.134).abs() < 1e-3, "{}", ring.extent());
}

#[test]
fn local_to_world_follows_host_spin() {
    let center = Vector3::new(2.0, 0.5, -1.0);
    let pose = |yaw| HostPose {
        transform: Transform {
            yaw,
            ..Transform::new(center, 1.3)
        },
        radius: 1.3 * 0.866,
    };
    let local = Vector3::new(0.8, 0.3, 0.0);
    // Same as the host's own model matrix.
    let t = pose(0.4).attachment_transform(0.0);
    let m = t.matrix();
    let by_matrix = Vector3::new(
        m.m0 * local.x + m.m4 * local.y + m.m8 * local.z + m.m12,
        m.m1 * local.x + m.m5 * local.y + m.m9 * local.z + m.m13,
        m.m2 * local.x + m.m6 * local.y + m.m10 * local.z + m.m14,
    );
    assert!((pose(0.4).to_world(local, 0.0) - by_matrix).length() < 1e-5);
    // Turning the host by d turns the attachment's points by d about the
    // host's axis: distance to the center and height are kept.
    let a = pose(0.4).to_world(local, 0.0) - center;
    let b = pose(0.4 + 0.9).to_world(local, 0.0) - center;
    assert!((a.length() - b.length()).abs() < 1e-5);
    assert!((a.y - b.y).abs() < 1e-5);
    let turned = (a.x * b.x + a.z * b.z) / (a.x.hypot(a.z) * b.x.hypot(b.z));
    assert!((turned - 0.9f32.cos()).abs() < 1e-4);
    // Its own angle adds to the host's spin.
    assert!((pose(0.4).to_world(local, 0.9) - pose(1.3).to_world(local, 0.0)).length() < 1e-5);
}

//! Attachments bound to their host by id. The list order is the draw
//! order; hosts and duplicates are checked with clean errors.

use crate::pipeline::Transform;
use crate::solar::SPHERE_RADIUS;
use crate::solar::attach::props::PropLayers;
use crate::solar::card_ring::CardRing;
use crate::solar::royal_crown::RoyalCrown;
use crate::solar::system::{PlanetId, SpecError};
use raylib::math::Vector3;

/// What is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// The gold crown sitting on the star (at most one per host).
    RoyalCrown,
    /// The ring of diamond cards (at most one per host).
    CardRing,
    /// Surface props baked into sectors (`props::bake`, at most one set
    /// per host).
    Props,
}

/// One attachment and the id of the body it rides on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentSpec {
    pub kind: AttachmentKind,
    pub host: PlanetId,
}

/// Borrowed geometry of one attachment.
#[derive(Debug, Clone, Copy)]
pub enum Attachment<'a> {
    RoyalCrown(&'a RoyalCrown),
    CardRing(&'a CardRing),
    Props(&'a PropLayers),
}

impl Attachment<'_> {
    /// Conservative reach from the host center, in host radii: every
    /// vertex of the attachment lies inside this sphere.
    pub fn extent(&self) -> f32 {
        match self {
            Self::RoyalCrown(crown) => crown.extent,
            Self::CardRing(ring) => ring.extent(),
            // Baked on the unit-scale planet of radius `SPHERE_RADIUS`.
            Self::Props(props) => (props.reach() / SPHERE_RADIUS).max(1.0),
        }
    }
}

/// Where a host is this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostPose {
    /// The host's transform (center, spin and mesh scale).
    pub transform: Transform,
    /// Radius of the host, world units.
    pub radius: f32,
}

impl HostPose {
    /// Transform of an attachment that turns with the host plus its own
    /// `angle` (radians around the host's axis).
    pub fn attachment_transform(&self, angle: f32) -> Transform {
        Transform {
            yaw: self.transform.yaw + angle,
            ..self.transform
        }
    }

    /// World position of a point given in the attachment's object space.
    pub fn to_world(&self, local: Vector3, angle: f32) -> Vector3 {
        let t = self.attachment_transform(angle);
        t.position + t.to_world_dir(local) * t.scale
    }
}

/// Index of each attachment's host in `hosts` (the scene's body ids, in
/// order). An id missing from `hosts` is `UnknownHost`; a second
/// attachment of the same kind on the same host is `DuplicateId`.
pub fn bind_attachments(
    specs: &[AttachmentSpec],
    hosts: &[PlanetId],
) -> Result<Vec<usize>, SpecError> {
    specs
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            if specs[..i]
                .iter()
                .any(|s| s.kind == spec.kind && s.host == spec.host)
            {
                return Err(SpecError::DuplicateId(spec.host));
            }
            hosts
                .iter()
                .position(|&id| id == spec.host)
                .ok_or(SpecError::UnknownHost(spec.host))
        })
        .collect()
}

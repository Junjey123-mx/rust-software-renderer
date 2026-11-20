//! Geometry bound to a host body (the royal crown, the card ring): one
//! contract instead of an `Option` field per decoration.

pub mod landmark;
pub mod prop_meshes;
pub mod props;
pub mod spec;
pub mod waterfalls;

pub use spec::{Attachment, AttachmentKind, AttachmentSpec, HostPose, bind_attachments};

use crate::solar::system::SpecError;

/// Largest reach of `attachments` from the host center, host radii (0
/// without any). A reach that is not a finite positive number is an error.
pub fn max_extent(attachments: &[Attachment]) -> Result<f32, SpecError> {
    attachments.iter().try_fold(0.0, |max: f32, a| {
        let e = a.extent();
        if !e.is_finite() {
            return Err(SpecError::NonFinite("attachment.extent"));
        }
        if e <= 0.0 {
            return Err(SpecError::OutOfRange("attachment.extent", e));
        }
        Ok(max.max(e))
    })
}

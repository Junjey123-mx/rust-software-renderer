//! Geometry bound to a host body (the royal crown, the card ring): one
//! contract instead of an `Option` field per decoration.

pub mod spec;

pub use spec::{Attachment, AttachmentKind, AttachmentSpec, HostPose, bind_attachments};

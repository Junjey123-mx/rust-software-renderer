//! Rendering infrastructure.
//!
//! Fragment representation, framebuffer operations,
//! and presentation of generated pixels.

pub mod fragment;
pub mod framebuffer;
pub mod presenter;

pub use fragment::Fragment;
pub use framebuffer::Framebuffer;
pub use presenter::Presenter;

//! CPU rasterization algorithms.
//!
//! Point, line, and triangle fragment generation.
//! Laboratory 5 adds selectable filled modes.

pub mod line;
pub mod mode;
pub mod triangle;

pub use line::line;
pub use mode::TriangleMode;
pub use triangle::triangle;

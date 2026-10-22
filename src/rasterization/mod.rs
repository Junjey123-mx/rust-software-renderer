//! CPU rasterization algorithms.
//!
//! Point, line, and triangle fragment generation.
//! Laboratory 5 adds selectable filled modes.

pub mod coverage;
pub mod line;
pub mod mode;
pub mod triangle;

pub use coverage::RasterBounds;
pub use line::line;
pub use mode::TriangleMode;
pub use triangle::triangle;

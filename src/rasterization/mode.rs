//! Triangle rasterization modes required by laboratory 5.

use std::fmt;
use std::str::FromStr;

/// Which fragment-generation algorithm `rasterize` uses for each triangle.
/// Geometry, primitive assembly and fragment processing do not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TriangleMode {
    /// Laboratory 4 behaviour: three `line` calls, outline only.
    #[default]
    Wireframe,
    /// Bounding-box scan with barycentric weights at pixel centers.
    FilledBarycentric,
    /// Row-by-row edge intersections and horizontal spans.
    FilledScanline,
}

impl TriangleMode {
    pub const ALL: [TriangleMode; 3] = [
        Self::Wireframe,
        Self::FilledBarycentric,
        Self::FilledScanline,
    ];

    /// Command-line identifier.
    pub fn name(self) -> &'static str {
        match self {
            Self::Wireframe => "wireframe",
            Self::FilledBarycentric => "barycentric",
            Self::FilledScanline => "scanline",
        }
    }
}

impl fmt::Display for TriangleMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for TriangleMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|m| m.name() == s)
            .ok_or_else(|| {
                format!("unknown mode '{s}' (expected wireframe, barycentric or scanline)")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_wireframe() {
        assert_eq!(TriangleMode::default(), TriangleMode::Wireframe);
    }

    #[test]
    fn names_round_trip() {
        for m in TriangleMode::ALL {
            assert_eq!(m.name().parse::<TriangleMode>().unwrap(), m);
            assert_eq!(m.to_string(), m.name());
        }
        assert!("filled".parse::<TriangleMode>().is_err());
    }
}

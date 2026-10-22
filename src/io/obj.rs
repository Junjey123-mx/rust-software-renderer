//! Minimal triangulated OBJ loader.
//!
//! Reads only `v` position records and `f` triangular faces, as taught in the
//! course. Everything else (`vt`, `vn`, `mtllib`, `usemtl`, `o`, `s`, `g`,
//! comments) is ignored. Face corners may be `i`, `i/t`, `i//n` or `i/t/n`;
//! only the position index before the first `/` is used.

use crate::geometry::{Mesh, MeshError};
use raylib::math::Vector3;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ObjError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidVertex {
        line: usize,
        text: String,
    },
    NonTriangularFace {
        line: usize,
        corners: usize,
    },
    InvalidIndex {
        line: usize,
        token: String,
    },
    IndexOutOfRange {
        line: usize,
        index: usize,
        vertex_count: usize,
    },
    NoGeometry,
    Mesh(MeshError),
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::InvalidVertex { line, text } => {
                write!(f, "line {line}: invalid vertex record '{text}'")
            }
            Self::NonTriangularFace { line, corners } => write!(
                f,
                "line {line}: face has {corners} corners, only triangles are supported"
            ),
            Self::InvalidIndex { line, token } => write!(
                f,
                "line {line}: invalid vertex index '{token}' (expected a positive integer)"
            ),
            Self::IndexOutOfRange {
                line,
                index,
                vertex_count,
            } => write!(
                f,
                "line {line}: vertex index {index} exceeds the {vertex_count} positions in the file"
            ),
            Self::NoGeometry => write!(f, "file contains no triangular faces"),
            Self::Mesh(e) => write!(f, "invalid mesh: {e}"),
        }
    }
}

impl std::error::Error for ObjError {}

/// Reads and parses an OBJ file from disk.
pub fn load_obj(path: &Path) -> Result<Mesh, ObjError> {
    let source = std::fs::read_to_string(path).map_err(|source| ObjError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_obj(&source)
}

/// Parses OBJ text into an indexed mesh.
pub fn parse_obj(source: &str) -> Result<Mesh, ObjError> {
    let mut vertices: Vec<Vector3> = Vec::new();
    // (zero-based index, source line) so range errors can name the line.
    let mut indices: Vec<(usize, usize)> = Vec::new();

    for (number, raw) in source.lines().enumerate() {
        let line = number + 1;
        let mut tokens = raw.split_whitespace();
        match tokens.next() {
            Some("v") => {
                let coords: Vec<f32> = tokens
                    .take(4)
                    .map(|t| t.parse::<f32>().ok().filter(|c| c.is_finite()))
                    .collect::<Option<Vec<f32>>>()
                    .ok_or_else(|| ObjError::InvalidVertex {
                        line,
                        text: raw.trim().to_string(),
                    })?;
                // OBJ allows an optional fourth `w`; the course uses x y z.
                if coords.len() < 3 {
                    return Err(ObjError::InvalidVertex {
                        line,
                        text: raw.trim().to_string(),
                    });
                }
                vertices.push(Vector3::new(coords[0], coords[1], coords[2]));
            }
            Some("f") => {
                let corners: Vec<&str> = tokens.collect();
                if corners.len() != 3 {
                    return Err(ObjError::NonTriangularFace {
                        line,
                        corners: corners.len(),
                    });
                }
                for corner in corners {
                    let first = corner.split('/').next().unwrap_or("");
                    let obj_index: usize = match first.parse::<usize>() {
                        Ok(i) if i > 0 => i,
                        _ => {
                            return Err(ObjError::InvalidIndex {
                                line,
                                token: corner.to_string(),
                            });
                        }
                    };
                    indices.push((obj_index - 1, line));
                }
            }
            _ => {}
        }
    }

    if indices.is_empty() {
        return Err(ObjError::NoGeometry);
    }
    if let Some(&(index, line)) = indices.iter().find(|(i, _)| *i >= vertices.len()) {
        return Err(ObjError::IndexOutOfRange {
            line,
            index: index + 1,
            vertex_count: vertices.len(),
        });
    }

    Mesh::new(vertices, indices.into_iter().map(|(i, _)| i).collect()).map_err(ObjError::Mesh)
}

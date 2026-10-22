//! Model input and parsing.
//!
//! Responsible for reading triangulated OBJ files,
//! validating vertex indices, and producing meshes.

pub mod obj;

pub use obj::{ObjError, load_obj, parse_obj};

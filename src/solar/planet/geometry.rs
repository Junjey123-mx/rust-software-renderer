//! Planet geometry: the icosphere displaced by the planet's height field.
//! Each level of detail is a mesh of its own (no cracks between levels:
//! they are never drawn together), and the sea is part of the surface
//! (`HeightField::surface_radius`), not a second mesh.
//!
//! The levels are built once, at startup, through `GeometryCache`: every
//! instance with the same radius and field shares the same buffers, and
//! nothing is built while a frame is drawn.

use crate::geometry::Vertex;
use crate::solar::planet::HeightField;
use crate::solar::planet::icosphere::{icosphere, to_mesh};
use std::collections::HashMap;
use std::sync::Arc;

/// Icosphere levels kept for a relief planet, coarse to fine.
pub const LOD_LEVELS: [u8; 4] = [2, 3, 4, 5];

/// The meshes of `LOD_LEVELS`, in that order, shared by reference count.
pub type LodMeshes = Arc<[Arc<Vec<Vertex>>; 4]>;

/// The icosphere of `level` with every direction `n` at
/// `n * radius * field.surface_radius(n)`, in the pipeline's triangle list.
pub fn relief_mesh(level: u8, radius: f32, field: &HeightField) -> Result<Vec<Vertex>, String> {
    Ok(to_mesh(&icosphere(level)?, |n| {
        radius * field.surface_radius(n)
    }))
}

/// What a relief geometry depends on, as exact bits: the radius, the kind
/// of field and its parameters. Another radius or field is another key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometryKey {
    pub radius_bits: u32,
    pub field_tag: u8,
    pub field_bits: [u32; 3],
}

impl GeometryKey {
    pub fn new(radius: f32, field: &HeightField) -> Self {
        let (field_tag, field_bits) = match *field {
            HeightField::Flat => (0, [0; 3]),
            HeightField::Test(t) => (
                1,
                [
                    t.amplitude.to_bits(),
                    t.frequency.to_bits(),
                    t.sea_level.map_or(u32::MAX, f32::to_bits),
                ],
            ),
        };
        Self {
            radius_bits: radius.to_bits(),
            field_tag,
            field_bits,
        }
    }
}

/// Relief geometries by key, built on first request (at startup) and
/// shared afterwards. `builds` counts the keys built so far.
#[derive(Debug, Clone, Default)]
pub struct GeometryCache {
    map: HashMap<GeometryKey, LodMeshes>,
    builds: usize,
}

impl GeometryCache {
    /// The levels of detail of `field` at `radius`: the cached ones, or
    /// built now (all four) if this key is new.
    pub fn get_or_build(&mut self, radius: f32, field: &HeightField) -> Result<LodMeshes, String> {
        let key = GeometryKey::new(radius, field);
        if let Some(lods) = self.map.get(&key) {
            return Ok(Arc::clone(lods));
        }
        let mesh = |i: usize| relief_mesh(LOD_LEVELS[i], radius, field).map(Arc::new);
        let lods: LodMeshes = Arc::new([mesh(0)?, mesh(1)?, mesh(2)?, mesh(3)?]);
        self.map.insert(key, Arc::clone(&lods));
        self.builds += 1;
        Ok(lods)
    }

    /// Geometries built since the cache was created.
    pub fn builds(&self) -> usize {
        self.builds
    }

    /// Bytes held by the cached vertex lists.
    pub fn bytes(&self) -> usize {
        self.map
            .values()
            .flat_map(|lods| lods.iter())
            .map(|m| m.len() * size_of::<Vertex>())
            .sum()
    }
}

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
    field.validate()?;
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
            HeightField::Clover(c) => {
                let d = c.digest();
                (2, [c.seed, d as u32, (d >> 32) as u32])
            }
            HeightField::Hearts(h) => {
                let d = h.digest();
                (3, [h.seed, d as u32, (d >> 32) as u32])
            }
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

/// Relief of one body: its height field and its levels of detail.
#[derive(Debug, Clone)]
pub struct Relief {
    pub field: HeightField,
    pub lods: LodMeshes,
    /// Share of the field's normal in the shading (1: full relief, 0: the
    /// sphere's normal, to compare).
    pub bump: f32,
}

impl Relief {
    /// Mesh of `level` (one of `LOD_LEVELS`; out of range is clamped).
    pub fn mesh(&self, level: u8) -> &Arc<Vec<Vertex>> {
        let i = level.clamp(LOD_LEVELS[0], LOD_LEVELS[3]) - LOD_LEVELS[0];
        &self.lods[i as usize]
    }
}

/// Projected radius in pixels from which levels 3, 4 and 5 are used...
const LOD_ENTER_PX: [f32; 3] = [48.0, 140.0, 360.0];
/// ...and below which they fall back to 2, 3 and 4.
const LOD_EXIT_PX: [f32; 3] = [40.0, 120.0, 300.0];

/// Level of detail of one body across frames (live viewer only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodState {
    pub level: u8,
}

impl Default for LodState {
    fn default() -> Self {
        Self {
            level: LOD_LEVELS[0],
        }
    }
}

/// Level for a body `px` pixels in radius, without history: the level a
/// fresh `LodState` climbs to. Deterministic, for recording and benchmarks.
pub fn lod_for(px: f32) -> u8 {
    lod_step(&mut LodState::default(), px)
}

/// Level for `px` with hysteresis: it rises at 48, 140 and 360 px and only
/// falls below 40, 120 and 300 px, so a size that wavers around one bound
/// keeps its level. A non-finite size counts as 0 px.
pub fn lod_step(state: &mut LodState, px: f32) -> u8 {
    let px = if px.is_finite() { px } else { 0.0 };
    let (low, high) = (LOD_LEVELS[0], LOD_LEVELS[3]);
    state.level = state.level.clamp(low, high);
    let i = |level: u8| (level - low) as usize;
    while state.level < high && px >= LOD_ENTER_PX[i(state.level)] {
        state.level += 1;
    }
    while state.level > low && px < LOD_EXIT_PX[i(state.level) - 1] {
        state.level -= 1;
    }
    state.level
}

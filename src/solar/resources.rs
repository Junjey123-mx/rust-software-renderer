//! Geometry shared by every instance of the solar scene: the course sphere,
//! the spacecraft and its far dart, built once and never mutated, and the
//! relief planets' levels of detail, built at startup.

use crate::config::{AppConfig, SPHERE_MODEL_PATH};
use crate::geometry::Vertex;
use crate::io::load_obj;
use crate::solar::attach::prop_meshes::lightcycle_mesh;
use crate::solar::attach::traffic::dart_mesh;
use crate::solar::planet::geometry::GeometryCache;
use raylib::math::Vector3;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

/// Distance from the center to the farthest vertex of `sphere.obj` (its
/// vertices lie between 0.840 and 0.866): the body radius at scale 1.
pub const SPHERE_RADIUS: f32 = 0.866;

/// Expanded vertex lists (one `Vertex` per triangle corner), shared through
/// reference counting. The corner color is neutral white; any wireframe
/// accent is applied per instance, never by mutating these lists.
#[derive(Debug, Clone)]
pub struct SolarMeshes {
    pub sphere: Arc<Vec<Vertex>>,
    pub ship: Arc<Vec<Vertex>>,
    /// The far stand-in of the neon ships (`traffic::dart_mesh`), shared
    /// by all of them.
    pub dart: Arc<Vec<Vertex>>,
    /// The Hearts lightcycle (`prop_meshes::lightcycle_mesh`, bounding
    /// radius 1, base at the origin), shared by every cycle.
    pub lightcycle: Arc<Vec<Vertex>>,
    /// Relief geometries by radius and field (empty until a planet has a
    /// height field).
    pub relief: GeometryCache,
}

impl SolarMeshes {
    /// Loads `sphere.obj` and the spacecraft OBJ from the crate assets.
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let sphere = load_obj(Path::new(SPHERE_MODEL_PATH))?;
        let ship = load_obj(&AppConfig::default().model_path)?;
        Ok(Self {
            sphere: Arc::new(sphere.expand_vertices(Vector3::one())),
            ship: Arc::new(ship.expand_vertices(Vertex::DEFAULT_COLOR)),
            dart: Arc::new(dart_mesh().expand_vertices(Vertex::DEFAULT_COLOR)),
            lightcycle: Arc::new(lightcycle_mesh()?.expand_vertices(Vertex::DEFAULT_COLOR)),
            relief: GeometryCache::default(),
        })
    }
}

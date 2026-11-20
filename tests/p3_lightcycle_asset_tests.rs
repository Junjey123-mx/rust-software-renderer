//! Project 3 lightcycle (C66): an original procedural low-poly model (the
//! catalog's "OBJ procedural autoral" alternative: no external file),
//! loaded once into the shared meshes, nose to -Z and top to +Y like the
//! spacecraft, base on the ground and bounding radius 1.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::io::parse_obj;
use rust_software_renderer::solar::SolarMeshes;
use rust_software_renderer::solar::attach::prop_meshes::{
    LIGHTCYCLE_FORWARD, LIGHTCYCLE_UP, lightcycle_mesh, normalize_model,
};

fn bounds(m: &Mesh) -> (Vector3, Vector3) {
    m.vertices.iter().fold(
        (Vector3::one() * f32::MAX, Vector3::one() * f32::MIN),
        |(lo, hi), p| {
            (
                Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        },
    )
}

#[test]
fn parses_non_empty() {
    let m = lightcycle_mesh().unwrap();
    assert_eq!(m.indices.len() / 3, 82);
    assert!(m.indices.iter().all(|&i| i < m.vertices.len()));
    assert!(
        m.vertices
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
    );
    // Loaded once with the other shared meshes.
    let meshes = SolarMeshes::load().unwrap();
    assert_eq!(meshes.lightcycle.len(), m.indices.len());
    // Closed and wound outwards: positive signed volume about its middle.
    let (lo, hi) = bounds(&m);
    let c = (lo + hi) * 0.5;
    let volume: f32 = m
        .indices
        .chunks(3)
        .map(|t| {
            let [a, b, d] = [0, 1, 2].map(|k| m.vertices[t[k]] - c);
            a.dot(b.cross(d)) / 6.0
        })
        .sum();
    assert!(volume > 0.0, "{volume}");
}

#[test]
fn forward_and_up_convention() {
    assert_eq!(LIGHTCYCLE_FORWARD, Vector3::new(0.0, 0.0, -1.0));
    assert_eq!(LIGHTCYCLE_UP, Vector3::new(0.0, 1.0, 0.0));
    let m = lightcycle_mesh().unwrap();
    let (lo, hi) = bounds(&m);
    let size = hi - lo;
    // Long along the heading, standing on +Y.
    assert!(size.z > 2.0 * size.y && size.z > 4.0 * size.x, "{size:?}");
    assert!(lo.y.abs() < 1e-6);
    // The nose is a single tip at -Z; the tail is flat at +Z.
    let at = |z: f32| m.vertices.iter().filter(|p| (p.z - z).abs() < 1e-5).count();
    assert_eq!(at(lo.z), 1, "one nose tip");
    assert!(at(hi.z) >= 4, "a flat tail");
}

#[test]
fn scale_normalized() {
    let m = lightcycle_mesh().unwrap();
    let (lo, hi) = bounds(&m);
    let c = (lo + hi) * 0.5;
    let radius = m
        .vertices
        .iter()
        .map(|p| (*p - c).length())
        .fold(0.0, f32::max);
    assert!((radius - 1.0).abs() < 1e-5, "{radius}");
    assert!(c.x.abs() < 1e-5 && c.z.abs() < 1e-5, "footprint centered");
}

#[test]
fn empty_or_degenerate_is_an_error() {
    let empty = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    assert!(normalize_model(empty).is_err());
    let flat = Mesh {
        vertices: vec![
            Vector3::zero(),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
        ],
        indices: vec![0, 1, 2],
    };
    assert!(normalize_model(flat).is_err());
    let past = Mesh {
        vertices: vec![Vector3::zero()],
        indices: vec![0, 1, 2],
    };
    assert!(normalize_model(past).is_err());
    // The OBJ loader refuses an empty file the same way, without a panic.
    assert!(parse_obj("").is_err());
    assert!(parse_obj("# nothing here\n").is_err());
}

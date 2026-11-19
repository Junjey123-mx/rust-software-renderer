//! The spacecraft exhaust: one shared flame at each measured nozzle.

use raylib::math::{Matrix, Vector3};
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::io::obj::load_obj;
use rust_software_renderer::solar::ShipState;
use rust_software_renderer::solar::thrusters::{
    INSET, NOZZLES, Thrusters, flame_matrix, flame_mesh,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

fn apply(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

fn close(a: Vector3, b: Vector3, tol: f32) -> bool {
    (a - b).length() < tol
}

fn is_closed(mesh: &Mesh) -> bool {
    let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
    for t in mesh.indices.chunks(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a, b)).or_default() += 1;
        }
    }
    edges
        .iter()
        .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
}

#[test]
fn every_nozzle_sits_on_a_square_mouth_of_the_ship_mesh() {
    let ship = load_obj(Path::new("assets/models/Laboratorio3.obj")).unwrap();
    for n in NOZZLES {
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
            let corner = n.center + Vector3::new(sx * n.half, sy * n.half, 0.0);
            assert!(
                ship.vertices.iter().any(|&v| close(v, corner, 1e-3)),
                "no mouth corner at {corner:?}"
            );
        }
        let reach = n.half + 0.1;
        let behind = ship.vertices.iter().filter(|v| {
            v.z > n.center.z + 1e-3 && (v.x - n.center.x).abs() < reach && v.y.abs() < reach
        });
        assert_eq!(
            behind.count(),
            0,
            "nothing covers the mouth at {:?}",
            n.center
        );
    }
    assert_eq!(NOZZLES[1].center.x, -NOZZLES[2].center.x, "symmetric pods");
}

#[test]
fn the_flame_is_a_closed_outward_solid_from_a_square_base_to_a_tip() {
    let mesh = flame_mesh();
    assert!(is_closed(&mesh));
    let interior = Vector3::new(0.0, 0.0, 0.3);
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (
            mesh.vertices[t[0]],
            mesh.vertices[t[1]],
            mesh.vertices[t[2]],
        );
        let n = (b - a).cross(c - a);
        assert!(n.dot((a + b + c) / 3.0 - interior) > 0.0, "inward face");
    }
    let base: Vec<_> = mesh.vertices.iter().filter(|v| v.z == 0.0).collect();
    assert_eq!(base.len(), 8);
    assert!(
        base.iter()
            .all(|v| (v.x.abs().max(v.y.abs()) - 1.0).abs() < 1e-5)
    );
    assert!(mesh.vertices.iter().all(|v| (0.0..=1.0).contains(&v.z)));
    assert!(mesh.vertices.contains(&Vector3::new(0.0, 0.0, 1.0)), "tip");
    assert_eq!(mesh.indices.len() / 3, 62);
}

#[test]
fn every_nozzle_shares_one_flame_mesh() {
    let t = Thrusters::default();
    assert!(Arc::ptr_eq(&t.mesh, &t.clone().mesh));
    assert_eq!(t.mesh.len(), flame_mesh().indices.len());
}

#[test]
fn to_world_matches_the_ship_model_matrix() {
    let mut s = ShipState::new(Vector3::new(1.0, -2.0, 3.0), 0.08);
    s.yaw(0.7);
    s.pitch(-0.4);
    let m = s.model_matrix();
    for p in [
        Vector3::zero(),
        Vector3::new(2.45, 0.3, 1.57),
        Vector3::new(-1.0, 2.0, -3.0),
    ] {
        assert!(close(apply(&m, p), s.to_world(p), 1e-5));
    }
}

#[test]
fn each_flame_rides_on_its_nozzle_and_points_out_of_the_back() {
    let mut s = ShipState::new(Vector3::new(4.0, 0.5, -2.0), 0.08);
    s.yaw(1.1);
    s.pitch(0.3);
    let length = 1.6;
    for n in NOZZLES {
        let m = flame_matrix(&s, &n, length);
        let base = s.to_world(n.center - Vector3::new(0.0, 0.0, INSET));
        assert!(close(apply(&m, Vector3::zero()), base, 1e-5));
        let tip = apply(&m, Vector3::new(0.0, 0.0, 1.0));
        let axis = tip - base;
        assert!(
            close(axis.normalized(), -s.forward, 1e-5),
            "out of the back"
        );
        assert!((axis.length() - (length + INSET) * s.scale).abs() < 1e-5);
        let corner = apply(&m, Vector3::new(1.0, 1.0, 0.0));
        let mouth = s.to_world(n.center + Vector3::new(n.half, n.half, -INSET));
        assert!(close(corner, mouth, 1e-5), "the base fills the mouth");
    }
    let mut moved = s;
    moved.position += Vector3::new(0.0, 1.0, 0.0);
    let (a, b) = (
        flame_matrix(&s, &NOZZLES[0], length),
        flame_matrix(&moved, &NOZZLES[0], length),
    );
    assert!((b.m13 - a.m13 - 1.0).abs() < 1e-5 && (b.m12 - a.m12).abs() < 1e-6);
}

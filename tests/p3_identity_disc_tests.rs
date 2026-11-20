//! Project 3 Hearts identity disc: a 3D heart (extruded parametric outline
//! with a halo ring), upright over the pyramid's top, turning about the
//! surface normal and with the planet.

use raylib::math::{Matrix, Vector3};
use rust_software_renderer::pipeline::Transform;
use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::attach::landmark::{DISC_LIFT, DISC_SPIN, Landmark};
use rust_software_renderer::solar::attach::prop_meshes::{heart_disc_mesh, heart_outline};
use rust_software_renderer::solar::planet::{HeartsField, HeightField};

fn landmark() -> Landmark {
    let field = HeightField::Hearts(HeartsField::new(11));
    Landmark::pyramid(&field, Vector3::new(0.3, 0.8, 0.5), SPHERE_RADIUS).unwrap()
}

fn apply(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

#[test]
fn is_3d_not_sprite() {
    let m = heart_disc_mesh(20).unwrap();
    assert_eq!(m.indices.len() / 3, 4 * 20 + 80);
    let depth = m.vertices.iter().map(|v| v.z).fold(f32::MIN, f32::max)
        - m.vertices.iter().map(|v| v.z).fold(f32::MAX, f32::min);
    assert!(depth > 0.15, "{depth}");
    // Side walls (normals in the heart's plane) join the two faces.
    let sides = m
        .indices
        .chunks(3)
        .filter(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| m.vertices[t[k]]);
            (b - a).cross(c - a).normalized().z.abs() < 0.1
        })
        .count();
    assert!(sides >= 2 * 20, "{sides}");
    // Each face covers the outline exactly once (the fan from the centroid
    // does not fold over the notch): its area is the outline's area.
    let outline = heart_outline(20);
    let area: f32 = (0..20)
        .map(|k| {
            let (a, b) = (outline[k], outline[(k + 1) % 20]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f32>()
        .abs()
        / 2.0;
    let front: f32 = m.indices[..]
        .chunks(3)
        .map(|t| [0, 1, 2].map(|k| m.vertices[t[k]]))
        .filter(|[a, b, c]| a.z > 0.08 && b.z > 0.08 && c.z > 0.08)
        .map(|[a, b, c]| (b - a).cross(c - a).length() / 2.0)
        .sum();
    assert!(
        (front - area).abs() < 1e-3 * area.max(1.0),
        "{front} vs {area}"
    );
}

#[test]
fn sits_on_pyramid_top() {
    let l = landmark();
    let lift = (l.disc_center - l.top_local).length() / SPHERE_RADIUS;
    assert!((lift - DISC_LIFT).abs() < 1e-4);
    assert!((l.disc_center - l.top_local).normalized().dot(l.up) > 0.9999);
    // With the host at the origin, every vertex of the disc is over the
    // pyramid's top, at any turn.
    let host = Transform::new(Vector3::zero(), 1.0);
    for t in [0.0, 0.7, 2.1, 5.0] {
        let m = l.disc_matrix(&host, t);
        for v in l.disc.iter() {
            let p = apply(&m, v.position);
            assert!((p - l.top_local).dot(l.up) > 0.0, "t {t}");
        }
    }
    assert!(
        l.reach > l.disc_center.length(),
        "the disc counts in the extent"
    );
}

#[test]
fn follows_host_rotation() {
    let l = landmark();
    for yaw in [0.0, 1.1, 2.7] {
        let host = Transform {
            yaw,
            ..Transform::new(Vector3::new(4.0, 0.5, -2.0), 1.3)
        };
        let m = l.disc_matrix(&host, 0.0);
        let origin = Vector3::new(m.m12, m.m13, m.m14);
        let expected = host.position + host.to_world_dir(l.disc_center) * host.scale;
        assert!((origin - expected).length() < 1e-4, "yaw {yaw}");
        let up = Vector3::new(m.m4, m.m5, m.m6).normalized();
        assert!(up.dot(host.to_world_dir(l.up).normalized()) > 0.9999);
    }
    // It turns DISC_SPIN rad/s about the normal.
    let host = Transform::new(Vector3::zero(), 1.0);
    let x = |t| {
        let m = l.disc_matrix(&host, t);
        Vector3::new(m.m0, m.m1, m.m2).normalized()
    };
    let turned = x(0.0).dot(x(1.0)).clamp(-1.0, 1.0).acos();
    assert!((turned - DISC_SPIN).abs() < 1e-3, "{turned}");
}

#[test]
fn too_few_samples_refused() {
    assert!(heart_disc_mesh(7).is_err());
    assert!(heart_disc_mesh(0).is_err());
    assert!(heart_disc_mesh(8).is_ok());
}

//! Perspective camera and projection.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{ClipVertex, PerspectiveCamera};
use std::f32::consts::FRAC_PI_2;

const W: u32 = 800;
const H: u32 = 600;

fn cam() -> PerspectiveCamera {
    PerspectiveCamera::new(
        Vector3::new(0.0, 0.0, -10.0),
        Vector3::zero(),
        Vector3::new(0.0, 1.0, 0.0),
        FRAC_PI_2,
        0.1,
    )
    .unwrap()
}

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn the_target_projects_to_the_screen_center_with_its_distance_as_depth() {
    let p = cam()
        .project_view(cam().world_to_view(Vector3::zero()), W, H)
        .unwrap();
    assert!(close(p, Vector3::new(400.0, 300.0, 10.0)), "{p:?}");
}

#[test]
fn view_basis_is_orthonormal_and_z_is_positive_in_front() {
    let b = cam().basis();
    assert!(close(b.forward, Vector3::new(0.0, 0.0, 1.0)));
    assert!(close(b.up, Vector3::new(0.0, 1.0, 0.0)));
    assert!(close(b.right, Vector3::new(-1.0, 0.0, 0.0)));
    assert!(b.right.dot(b.up).abs() < 1e-6 && b.up.dot(b.forward).abs() < 1e-6);
    assert!(cam().world_to_view(Vector3::new(0.0, 0.0, -20.0)).z < 0.0);
}

#[test]
fn a_point_twice_as_far_moves_half_as_much_on_screen() {
    let c = cam();
    let near = c
        .project_view(c.world_to_view(Vector3::new(0.0, 1.0, 0.0)), W, H)
        .unwrap();
    let far = c
        .project_view(c.world_to_view(Vector3::new(0.0, 1.0, 10.0)), W, H)
        .unwrap();
    // 90 degree vertical fov: focal = 300 px; 1 unit up at 10 units = 30 px up.
    assert!((near.y - 270.0).abs() < 1e-2, "{near:?}");
    assert!((far.y - 285.0).abs() < 1e-2, "{far:?}");
    assert!(far.z > near.z);
    // Same focal horizontally (square pixels); world +X is screen left here.
    let side = c
        .project_view(c.world_to_view(Vector3::new(1.0, 0.0, 0.0)), W, H)
        .unwrap();
    assert!((side.x - 370.0).abs() < 1e-2, "{side:?}");
}

#[test]
fn a_narrower_field_of_view_magnifies() {
    let wide = cam();
    let narrow = PerspectiveCamera { fov_y: 0.5, ..wide };
    let p = Vector3::new(0.0, 1.0, 0.0);
    let dw = (wide.project_view(wide.world_to_view(p), W, H).unwrap().y - 300.0).abs();
    let dn = (narrow
        .project_view(narrow.world_to_view(p), W, H)
        .unwrap()
        .y
        - 300.0)
        .abs();
    assert!(dn > dw * 3.0, "{dn} vs {dw}");
}

#[test]
fn points_in_front_of_the_near_plane_or_non_finite_are_rejected() {
    let c = cam();
    assert!(c.project_view(Vector3::new(0.0, 0.0, 0.05), W, H).is_none());
    assert!(c.project_view(Vector3::new(0.0, 0.0, -1.0), W, H).is_none());
    assert!(
        c.project_view(Vector3::new(f32::NAN, 0.0, 5.0), W, H)
            .is_none()
    );
    assert!(c.project_view(Vector3::new(0.0, 0.0, 0.1), W, H).is_some());
}

#[test]
fn degenerate_cameras_are_rejected_and_parallel_up_gets_a_fallback() {
    let o = Vector3::zero();
    let y = Vector3::new(0.0, 1.0, 0.0);
    assert!(PerspectiveCamera::new(o, o, y, 1.0, 0.1).is_err());
    assert!(PerspectiveCamera::new(o, y, Vector3::zero(), 1.0, 0.1).is_err());
    assert!(PerspectiveCamera::new(o, y, y, 0.0, 0.1).is_err());
    assert!(PerspectiveCamera::new(o, y, y, 1.0, 0.0).is_err());
    assert!(PerspectiveCamera::new(Vector3::new(f32::NAN, 0.0, 0.0), y, y, 1.0, 0.1).is_err());
    // Looking straight down with up = +Y: still a valid orthonormal basis.
    let down = PerspectiveCamera::new(Vector3::new(0.0, 5.0, 0.0), o, y, 1.0, 0.1).unwrap();
    let b = down.basis();
    assert!((b.right.length() - 1.0).abs() < 1e-5 && (b.up.length() - 1.0).abs() < 1e-5);
    assert!(b.right.dot(b.up).abs() < 1e-5);
}

#[test]
fn clip_vertex_keeps_every_attribute_and_interpolates_them() {
    let c = cam();
    let v = Vertex {
        position: Vector3::new(0.3, 0.4, 0.5),
        transformed_position: Vector3::new(1.0, 2.0, 3.0),
        color: Vector3::new(0.1, 0.2, 0.3),
    };
    let a = ClipVertex::from_world(&v, &c);
    assert_eq!(a.object_position, v.position);
    assert_eq!(a.world_position, v.transformed_position);
    assert!(close(
        a.view_position,
        c.world_to_view(v.transformed_position)
    ));
    let b = ClipVertex {
        object_position: Vector3::new(1.3, 1.4, 1.5),
        world_position: Vector3::new(3.0, 4.0, 5.0),
        view_position: Vector3::new(0.0, 0.0, 20.0),
        color: Vector3::one(),
    };
    let m = a.lerp(&b, 0.5);
    assert!(close(m.object_position, Vector3::new(0.8, 0.9, 1.0)));
    assert!(close(m.world_position, Vector3::new(2.0, 3.0, 4.0)));
    assert!(close(m.color, Vector3::new(0.55, 0.6, 0.65)));
    let s = m.to_screen(&c, W, H).unwrap();
    assert_eq!(s.position, m.object_position, "object position survives");
    assert!((s.transformed_position.z - m.view_position.z).abs() < 1e-5);
    let behind = ClipVertex {
        view_position: Vector3::new(0.0, 0.0, -1.0),
        ..b
    };
    assert!(behind.to_screen(&c, W, H).is_none());
}

#[test]
fn view_direction_points_from_the_fragment_to_the_eye() {
    let c = cam();
    let d = c.view_direction(Vector3::new(0.0, 0.0, 0.0));
    assert!(close(d, Vector3::new(0.0, 0.0, -1.0)));
    assert!((c.view_direction(Vector3::new(3.0, 4.0, 2.0)).length() - 1.0).abs() < 1e-5);
    assert!(
        close(c.view_direction(c.eye), -c.basis().forward),
        "at the eye: backwards"
    );
}

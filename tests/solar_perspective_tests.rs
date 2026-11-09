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

// ---------------------------------------------------------------------------
// Near-plane clipping
// ---------------------------------------------------------------------------

use rust_software_renderer::pipeline::clip_near;

fn cv(z: f32, tag: f32) -> ClipVertex {
    ClipVertex {
        object_position: Vector3::new(tag, 0.0, 0.0),
        world_position: Vector3::new(0.0, tag, 0.0),
        view_position: Vector3::new(tag, tag, z),
        color: Vector3::one() * tag,
    }
}

#[test]
fn a_triangle_fully_in_front_is_returned_whole() {
    let tri = [cv(1.0, 0.0), cv(2.0, 1.0), cv(3.0, 2.0)];
    let p = clip_near(&tri, 0.5);
    assert_eq!(p.len(), 3);
    assert_eq!(p.corners(), &tri);
    assert_eq!(p.triangles().count(), 1);
    assert_eq!(p.edges().count(), 3);
}

#[test]
fn a_triangle_fully_behind_or_non_finite_is_dropped() {
    let p = clip_near(&[cv(-1.0, 0.0), cv(-2.0, 1.0), cv(0.2, 2.0)], 0.5);
    assert!(p.is_empty() && p.triangles().count() == 0 && p.edges().count() == 0);
    let nan = [cv(f32::NAN, 0.0), cv(2.0, 1.0), cv(3.0, 2.0)];
    assert!(clip_near(&nan, 0.5).is_empty());
}

#[test]
fn one_corner_behind_gives_a_quad_and_two_triangles() {
    // Corner 0 at z = -1, the others at z = 1; near 0 -> cuts at t = 0.5.
    let tri = [cv(-1.0, 0.0), cv(1.0, 1.0), cv(1.0, 2.0)];
    let p = clip_near(&tri, 0.0);
    assert_eq!(p.len(), 4);
    assert_eq!(p.triangles().count(), 2);
    assert_eq!(p.edges().count(), 4, "perimeter only, no fan diagonal");
    let cut: Vec<&ClipVertex> = p
        .corners()
        .iter()
        .filter(|c| c.view_position.z == 0.0)
        .collect();
    assert_eq!(cut.len(), 2);
    for c in cut {
        // Interpolated attributes: halfway between corner 0 and 1 or 2.
        assert!(
            c.object_position.x == 0.5 || c.object_position.x == 1.0,
            "{c:?}"
        );
        assert_eq!(c.world_position.y, c.object_position.x);
        assert_eq!(c.color.x, c.object_position.x);
    }
    // Every produced triangle has all corners at or beyond the near plane.
    for t in p.triangles() {
        assert!(t.iter().all(|v| v.view_position.z >= 0.0));
    }
}

#[test]
fn two_corners_behind_give_one_smaller_triangle() {
    let tri = [cv(-1.0, 0.0), cv(-1.0, 1.0), cv(1.0, 2.0)];
    let p = clip_near(&tri, 0.0);
    assert_eq!(p.len(), 3);
    assert_eq!(p.triangles().count(), 1);
    let survivors: Vec<_> = p
        .corners()
        .iter()
        .filter(|c| c.view_position.z > 0.0)
        .collect();
    assert_eq!(survivors.len(), 1);
    assert_eq!(survivors[0].object_position.x, 2.0);
}

#[test]
fn a_corner_exactly_on_the_near_plane_is_kept_without_duplicates() {
    let tri = [cv(0.5, 0.0), cv(2.0, 1.0), cv(3.0, 2.0)];
    let p = clip_near(&tri, 0.5);
    assert_eq!(p.len(), 3);
    let tri = [cv(0.5, 0.0), cv(-2.0, 1.0), cv(3.0, 2.0)];
    let p = clip_near(&tri, 0.5);
    assert_eq!(p.len(), 4);
    assert!(p.corners().iter().all(|c| c.view_position.z >= 0.5));
}

// ---------------------------------------------------------------------------
// Shaded stage through the perspective projection
// ---------------------------------------------------------------------------

use rust_software_renderer::celestial::{Body, CelestialScene};
use rust_software_renderer::pipeline::{
    FragmentView, OrbitCamera, OrbitView, ShadedOptions, ShadedStats, SurfaceSample, Transform,
    Uniforms, ViewPreset, ViewProjection, Viewport, render_shaded, render_shaded_with,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SolarMeshes;

fn sphere() -> Vec<Vertex> {
    SolarMeshes::load().unwrap().sphere.as_ref().clone()
}

fn fnv(fb: &Framebuffer) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in fb.to_rgba_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn options(threads: usize) -> ShadedOptions {
    ShadedOptions {
        threads,
        ..ShadedOptions::default()
    }
}

fn eye_at(z: f32) -> PerspectiveCamera {
    PerspectiveCamera::new(
        Vector3::new(0.0, 0.0, z),
        Vector3::zero(),
        Vector3::new(0.0, 1.0, 0.0),
        FRAC_PI_2,
        0.1,
    )
    .unwrap()
}

fn normal_color(s: &SurfaceSample, _v: &FragmentView) -> Vector3 {
    (s.normal + Vector3::one()) * 0.5
}

fn draw_sphere<S>(
    transform: Transform,
    proj: ViewProjection,
    mode: TriangleMode,
    threads: usize,
    shader: &S,
) -> (Framebuffer, ShadedStats)
where
    S: Fn(&SurfaceSample, &FragmentView) -> Vector3 + Sync,
{
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let uniforms = Uniforms {
        model_matrix: transform.matrix(),
    };
    let stats = render_shaded_with(
        &mut fb,
        &uniforms,
        &sphere(),
        &proj,
        mode,
        options(threads),
        shader,
    )
    .unwrap();
    (fb, stats)
}

#[test]
fn orbit_projection_through_render_shaded_with_is_bit_identical_to_render_shaded() {
    for body in Body::ALL {
        let mut scene = CelestialScene::new(body, ViewPreset::Oblique, W, H).unwrap();
        scene.time = 6.0;
        let prepared = scene.material.prepare(&scene.uniforms(), scene.debug);
        let mut a = Framebuffer::new(W, H);
        a.clear();
        render_shaded(
            &mut a,
            &Uniforms::default(),
            &scene.vertices,
            scene.view,
            TriangleMode::FilledScanline,
            options(1),
            &|s| prepared.shade(s),
        )
        .unwrap();
        let mut b = Framebuffer::new(W, H);
        b.clear();
        render_shaded_with(
            &mut b,
            &Uniforms::default(),
            &scene.vertices,
            &ViewProjection::Orbit(scene.view),
            TriangleMode::FilledScanline,
            options(1),
            &|s, v| {
                assert_eq!(v.view_direction, scene.view.camera.eye_direction());
                prepared.shade(s)
            },
        )
        .unwrap();
        assert_eq!(fnv(&a), fnv(&b), "{body}");
    }
}

#[test]
fn a_nearer_sphere_covers_more_pixels_and_the_center_normal_faces_the_eye() {
    let proj = ViewProjection::Perspective(eye_at(-10.0));
    let (near_fb, near) = draw_sphere(
        Transform::new(Vector3::new(0.0, 0.0, -4.0), 1.0),
        proj,
        TriangleMode::FilledScanline,
        1,
        &normal_color,
    );
    let (_, far) = draw_sphere(
        Transform::new(Vector3::new(0.0, 0.0, 6.0), 1.0),
        proj,
        TriangleMode::FilledScanline,
        1,
        &normal_color,
    );
    assert!(
        near.fragments_shaded > far.fragments_shaded * 4,
        "{near:?} vs {far:?}"
    );
    assert_eq!(near.triangles_processed, 192);
    assert!(
        near.triangles_culled >= 90,
        "back faces culled: {}",
        near.triangles_culled
    );
    // Encoded normal at the disk center: (0, 0, -1) towards the eye on -Z.
    let c = near_fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap();
    assert!((c - Vector3::new(0.5, 0.5, 0.0)).length() < 0.08, "{c:?}");
}

#[test]
fn perspective_depth_is_the_true_view_distance_of_the_surface() {
    let cam = eye_at(-10.0);
    let (fb, _) = draw_sphere(
        Transform::new(Vector3::zero(), 1.0),
        ViewProjection::Perspective(cam),
        TriangleMode::FilledScanline,
        1,
        &normal_color,
    );
    let d = fb.get_depth(W as i32 / 2, H as i32 / 2).unwrap();
    // The nearest facet of the 0.84-0.87 radius sphere sits about 9.15 away.
    assert!((9.05..=9.2).contains(&d), "{d}");
    // Depth grows towards the limb and stays finite everywhere it was written.
    let limb = fb.get_depth(W as i32 / 2 + 20, H as i32 / 2).unwrap();
    assert!(limb > d && limb.is_finite());
    assert_eq!(
        fb.get_depth(2, 2),
        Some(f32::INFINITY),
        "background untouched"
    );
}

#[test]
fn the_view_direction_varies_across_the_disk_only_in_perspective() {
    let spread = |proj: ViewProjection| {
        let (fb, _) = draw_sphere(
            Transform::new(Vector3::zero(), 2.0),
            proj,
            TriangleMode::FilledScanline,
            1,
            &|_s: &SurfaceSample, v: &FragmentView| (v.view_direction + Vector3::one()) * 0.5,
        );
        let c = fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap();
        let e = fb.get_pixel(W as i32 / 2 + 120, H as i32 / 2).unwrap();
        ((c - e).length(), c)
    };
    let (persp, center) = spread(ViewProjection::Perspective(eye_at(-4.0)));
    assert!(
        (center - Vector3::new(0.5, 0.5, 0.0)).length() < 0.02,
        "{center:?}"
    );
    assert!(persp > 0.1, "perspective spread {persp}");
    let cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let view = OrbitView::for_world(cam, 3.0, Viewport::new(W, H, 0.1)).unwrap();
    let (ortho, _) = spread(ViewProjection::Orbit(view));
    assert!(ortho < 1e-6, "orthographic spread {ortho}");
}

#[test]
fn perspective_occlusion_is_independent_of_draw_order_and_thread_count() {
    let proj = ViewProjection::Perspective(eye_at(-10.0));
    let near = Transform::new(Vector3::new(0.0, 0.0, -2.0), 0.6);
    let far = Transform::new(Vector3::new(0.0, 0.0, 3.0), 1.5);
    let tag = |t: Transform| move |_s: &SurfaceSample, _v: &FragmentView| Vector3::one() * t.scale;
    let render = |order: [Transform; 2], threads: usize| {
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        for t in order {
            let uniforms = Uniforms {
                model_matrix: t.matrix(),
            };
            render_shaded_with(
                &mut fb,
                &uniforms,
                &sphere(),
                &proj,
                TriangleMode::FilledBarycentric,
                options(threads),
                &tag(t),
            )
            .unwrap();
        }
        fb
    };
    let a = render([far, near], 1);
    let b = render([near, far], 1);
    let c = render([near, far], 0);
    assert_eq!(fnv(&a), fnv(&b));
    assert_eq!(fnv(&b), fnv(&c));
    let center = a.get_pixel(W as i32 / 2, H as i32 / 2).unwrap();
    assert_eq!(
        center,
        Vector3::one() * 0.6,
        "the near sphere owns the center"
    );
    // Far sphere: radius 1.3 at distance 13 -> 30 px; near one: 0.52 at 8 -> 19 px.
    assert_eq!(
        a.get_pixel(W as i32 / 2 + 25, H as i32 / 2).unwrap(),
        Vector3::one() * 1.5
    );
}

#[test]
fn geometry_crossing_the_near_plane_is_clipped_without_artifacts() {
    // The eye sits 0.5 from the center of a radius ~0.87 sphere: the front
    // half is behind the eye and the near plane cuts through the mesh. Seen
    // from inside, every face points away, so culling is off here.
    let cam = eye_at(-0.5);
    let inside = ShadedOptions {
        cull_back_faces: false,
        ..options(1)
    };
    let draw_inside = |mode: TriangleMode| {
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        let stats = render_shaded_with(
            &mut fb,
            &Uniforms::default(),
            &sphere(),
            &ViewProjection::Perspective(cam),
            mode,
            inside,
            &normal_color,
        )
        .unwrap();
        (fb, stats)
    };
    let (fb, stats) = draw_inside(TriangleMode::FilledScanline);
    assert!(stats.fragments_shaded > 1000, "{stats:?}");
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let d = fb.get_depth(x, y).unwrap();
            assert!(d.is_infinite() || d >= cam.near, "({x}, {y}) depth {d}");
        }
    }
    // Both fills agree on the clipped geometry and the outline draws too.
    let (bary, _) = draw_inside(TriangleMode::FilledBarycentric);
    assert_eq!(fnv(&fb), fnv(&bary));
    let (wire, ws) = draw_inside(TriangleMode::Wireframe);
    assert!(ws.fragments_generated > 0 && ws.fragments_shaded == 0);
    assert!(wire.count_non_background() > 0);
    assert!(
        wire.get_depth(W as i32 / 2, H as i32 / 2) == Some(f32::INFINITY),
        "no depth"
    );
}

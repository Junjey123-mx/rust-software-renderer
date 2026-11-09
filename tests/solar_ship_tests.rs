//! ShipState: basis conventions, stability and the model matrix.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{Uniforms, vertex_shader};
use rust_software_renderer::solar::SolarMeshes;
use rust_software_renderer::solar::ship::{MESH_CENTER, MESH_RADIUS, ShipState};
use std::f32::consts::FRAC_PI_2;

fn close(a: Vector3, b: Vector3) -> bool {
    (a - b).length() < 1e-4
}

fn orthonormal(s: &ShipState) -> bool {
    (s.forward.length() - 1.0).abs() < 1e-4
        && (s.up.length() - 1.0).abs() < 1e-4
        && s.forward.dot(s.up).abs() < 1e-4
}

#[test]
fn the_default_basis_has_the_nose_on_minus_z_and_the_top_on_plus_y() {
    let s = ShipState::new(Vector3::zero(), 1.0);
    assert!(close(s.forward, Vector3::new(0.0, 0.0, -1.0)));
    assert!(close(s.up, Vector3::new(0.0, 1.0, 0.0)));
    assert!(close(s.right(), Vector3::new(1.0, 0.0, 0.0)), "right is +X");
}

#[test]
fn positive_yaw_turns_the_nose_towards_minus_x_and_pitch_lifts_it() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.yaw(FRAC_PI_2);
    assert!(
        close(s.forward, Vector3::new(-1.0, 0.0, 0.0)),
        "{:?}",
        s.forward
    );
    assert!(close(s.up, Vector3::new(0.0, 1.0, 0.0)));
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.pitch(0.4);
    assert!(s.forward.y > 0.3 && s.forward.z < 0.0, "{:?}", s.forward);
    assert!(s.up.z > 0.0, "the top tilts backwards (+Z): {:?}", s.up);
    assert!(orthonormal(&s));
}

#[test]
fn ten_thousand_turns_keep_the_basis_orthonormal() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    for i in 0..10_000 {
        s.yaw(0.013 * ((i % 7) as f32 - 3.0));
        s.pitch(0.011 * ((i % 5) as f32 - 2.0));
    }
    assert!(orthonormal(&s));
    assert!((s.right().length() - 1.0).abs() < 1e-4);
}

#[test]
fn renormalize_recovers_from_a_degenerate_basis() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.up = s.forward;
    s.renormalize();
    assert!(orthonormal(&s));
    s.forward = Vector3::zero();
    s.renormalize();
    assert!(orthonormal(&s));
}

#[test]
fn the_model_matrix_places_the_mesh_center_at_the_position_and_the_nose_ahead() {
    let mut s = ShipState::new(Vector3::new(5.0, 1.0, -2.0), 0.1);
    s.yaw(FRAC_PI_2);
    let apply = |p: Vector3| {
        let u = Uniforms {
            model_matrix: s.model_matrix(),
        };
        vertex_shader(&Vertex::new(p), &u).transformed_position
    };
    assert!(close(apply(MESH_CENTER), s.position));
    // The measured nose tip of the OBJ, 3.1 ahead of the center.
    let nose = apply(Vector3::new(0.0, 0.0, -4.4));
    let expected = s.position + s.forward * 0.31 + s.up * (-0.255 * 0.1);
    assert!(close(nose, expected), "{nose:?} vs {expected:?}");
    // The real mesh: every corner lies within the scaled radius of the position.
    let meshes = SolarMeshes::load().unwrap();
    let far = meshes
        .ship
        .iter()
        .map(|v| (apply(v.position) - s.position).length())
        .fold(0.0, f32::max);
    assert!(far <= MESH_RADIUS * 0.1 + 1e-3, "{far}");
    assert!((s.radius() - 0.399).abs() < 1e-5);
}

#[test]
fn object_direction_mapping_inverts_the_basis() {
    let mut s = ShipState::new(Vector3::zero(), 1.0);
    s.yaw(0.8);
    s.pitch(-0.3);
    assert!(close(
        s.to_object_dir(s.forward),
        Vector3::new(0.0, 0.0, -1.0)
    ));
    assert!(close(s.to_object_dir(s.up), Vector3::new(0.0, 1.0, 0.0)));
    assert!(close(
        s.to_object_dir(s.right()),
        Vector3::new(1.0, 0.0, 0.0)
    ));
}

// ---------------------------------------------------------------------------
// render_ship
// ---------------------------------------------------------------------------

use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    PerspectiveCamera, ShadedOptions, Transform, ViewProjection,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::{BodyInstance, FrameContext, render_instance, render_ship};
use std::f32::consts::{FRAC_PI_3, PI};

const W: u32 = 320;
const H: u32 = 240;

fn camera() -> ViewProjection {
    ViewProjection::Perspective(
        PerspectiveCamera::new(
            Vector3::new(0.0, 0.0, -6.0),
            Vector3::zero(),
            Vector3::new(0.0, 1.0, 0.0),
            FRAC_PI_3,
            0.05,
        )
        .unwrap(),
    )
}

fn brightness(fb: &Framebuffer, x0: i32, x1: i32) -> f32 {
    let mut sum = 0.0;
    for y in 0..H as i32 {
        for x in x0..x1 {
            let p = fb.get_pixel(x, y).unwrap();
            sum += p.x + p.y + p.z;
        }
    }
    sum
}

fn top_camera() -> ViewProjection {
    ViewProjection::Perspective(
        PerspectiveCamera::new(
            Vector3::new(0.0, 6.0, 0.0),
            Vector3::zero(),
            Vector3::new(0.0, 0.0, -1.0),
            FRAC_PI_3,
            0.05,
        )
        .unwrap(),
    )
}

fn draw_ship(yaw: f32, light: Vector3) -> Framebuffer {
    let meshes = SolarMeshes::load().unwrap();
    let mut ship = ShipState::new(Vector3::zero(), 0.5);
    ship.yaw(yaw);
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    render_ship(
        &mut fb,
        &meshes.ship,
        &ship,
        &top_camera(),
        light,
        TriangleMode::FilledScanline,
        1,
    )
    .unwrap();
    fb
}

fn mean(fb: &Framebuffer) -> f32 {
    brightness(fb, 0, W as i32) / fb.count_non_background() as f32
}

#[test]
fn flat_lighting_turns_with_the_ship() {
    // Seen from straight above, turning the ship and its light together
    // around the vertical axis only rotates the image: the mean brightness
    // stays. Turning the ship alone under a fixed light changes which faces
    // are lit. Yaw +90 degrees maps object +X to world -Z.
    let still = mean(&draw_ship(0.0, Vector3::new(1.0, 0.0, 0.0)));
    let turned_with_light = mean(&draw_ship(PI / 2.0, Vector3::new(0.0, 0.0, -1.0)));
    let turned_alone = mean(&draw_ship(PI / 2.0, Vector3::new(1.0, 0.0, 0.0)));
    assert!(
        (still - turned_with_light).abs() < still * 0.02,
        "{still} vs {turned_with_light}"
    );
    assert!(
        (still - turned_alone).abs() > still * 0.05,
        "{still} vs {turned_alone}"
    );
    // A light from straight above brightens the deck more than a side light.
    assert!(mean(&draw_ship(0.0, Vector3::new(0.0, 1.0, 0.0))) > still * 1.5);
}

#[test]
fn the_ship_hides_a_planet_when_nearer_and_is_hidden_when_farther() {
    let meshes = SolarMeshes::load().unwrap();
    let planet = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.0),
        0.0,
    );
    let frame = FrameContext {
        projection: camera(),
        time: 1.0,
        light_world: Vector3::new(0.0, 0.0, -1.0),
        mode: TriangleMode::FilledScanline,
        options: ShadedOptions {
            threads: 1,
            ..ShadedOptions::default()
        },
    };
    let center = |fb: &Framebuffer| fb.get_pixel(W as i32 / 2, H as i32 / 2).unwrap();
    let mut only_planet = Framebuffer::new(W, H);
    only_planet.clear();
    render_instance(&mut only_planet, &meshes.sphere, &planet, &frame).unwrap();

    for (z, hidden) in [(-3.0, false), (3.0, true)] {
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        render_instance(&mut fb, &meshes.sphere, &planet, &frame).unwrap();
        let ship = ShipState::new(Vector3::new(0.0, 0.0, z), 0.08);
        let stats = render_ship(
            &mut fb,
            &meshes.ship,
            &ship,
            &camera(),
            frame.light_world,
            TriangleMode::FilledScanline,
            1,
        )
        .unwrap();
        assert_eq!(stats.triangles_processed, 708);
        assert_eq!(
            center(&fb) == center(&only_planet),
            hidden,
            "ship at z = {z}"
        );
    }
}

#[test]
fn ship_wireframe_outlines_without_writing_depth() {
    let meshes = SolarMeshes::load().unwrap();
    let ship = ShipState::new(Vector3::zero(), 0.5);
    let mut fb = Framebuffer::new(W, H);
    fb.clear();
    let stats = render_ship(
        &mut fb,
        &meshes.ship,
        &ship,
        &camera(),
        Vector3::new(0.0, 1.0, 0.0),
        TriangleMode::Wireframe,
        1,
    )
    .unwrap();
    assert!(stats.fragments_generated > 0 && stats.fragments_shaded == 0);
    assert!(fb.count_non_background() > 100);
    assert_eq!(
        fb.get_depth(W as i32 / 2, H as i32 / 2),
        Some(f32::INFINITY)
    );
}

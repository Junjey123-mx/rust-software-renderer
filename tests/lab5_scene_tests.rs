use raylib::math::Vector3;
use rust_software_renderer::app::{CliArgs, Scene, parse_args};
use rust_software_renderer::geometry::{SCENE_HEIGHT, SCENE_WIDTH, Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{Projection, Uniforms, render_with_mode};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::collections::HashSet;

fn args(list: &[&str]) -> Result<CliArgs, String> {
    parse_args(list.iter().map(|s| s.to_string()))
}

fn render_scene(mode: TriangleMode) -> (Framebuffer, usize) {
    let mut fb = Framebuffer::new(SCENE_WIDTH, SCENE_HEIGHT);
    let stats = render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &triangle_scene(),
        Projection::Screen,
        mode,
    )
    .unwrap();
    (fb, stats.triangles_processed)
}

#[test]
fn scene_geometry_is_deterministic_and_well_formed() {
    let a = triangle_scene();
    let b = triangle_scene();
    assert_eq!(a, b);
    assert_eq!(a.len(), 21, "seven triangles");
    assert!(a.iter().all(|v| v.position == v.transformed_position));
    assert!(
        a.iter()
            .all(|v| v.position.x.is_finite() && v.position.y.is_finite())
    );
    // One triangle intentionally exceeds the viewport; the rest are inside.
    let outside = a
        .iter()
        .filter(|v| v.position.x > SCENE_WIDTH as f32 || v.position.y > SCENE_HEIGHT as f32)
        .count();
    assert_eq!(outside, 2);
    // Each triangle has one color; the shared-edge pair has different colors.
    let colors: Vec<Vector3> = a.chunks_exact(3).map(|t| t[0].color).collect();
    assert!(
        a.chunks_exact(3)
            .all(|t| t[0].color == t[1].color && t[1].color == t[2].color)
    );
    assert_ne!(colors[4], colors[5]);
    assert_eq!(
        colors
            .iter()
            .map(|c| format!("{c:?}"))
            .collect::<HashSet<_>>()
            .len(),
        7
    );
}

#[test]
fn every_mode_processes_the_same_seven_triangles() {
    let (wire, n_w) = render_scene(TriangleMode::Wireframe);
    let (bary, n_b) = render_scene(TriangleMode::FilledBarycentric);
    let (scan, n_s) = render_scene(TriangleMode::FilledScanline);
    assert_eq!((n_w, n_b, n_s), (7, 7, 7));
    assert!(wire.count_non_background() > 0);
    assert!(bary.count_non_background() > wire.count_non_background() * 10);
    assert_eq!(
        bary.to_rgba_bytes(),
        scan.to_rgba_bytes(),
        "both fills paint identical images"
    );
    assert_ne!(bary.to_rgba_bytes(), wire.to_rgba_bytes());
    // Interior of the large triangle: empty in wireframe, filled otherwise.
    assert_eq!(wire.get_pixel(330, 220), Some(Vector3::zero()));
    assert_eq!(bary.get_pixel(330, 220), Some(Vector3::new(1.0, 0.75, 0.2)));
    // Shared diagonal pixels belong to exactly one of the two colors.
    for (x, y) in [(150, 410), (200, 460), (250, 510)] {
        let c = bary.get_pixel(x, y).unwrap();
        assert!(
            c == Vector3::new(0.2, 0.9, 0.9) || c == Vector3::new(0.9, 0.3, 0.9),
            "({x},{y}) = {c:?}"
        );
    }
}

#[test]
fn ship_scene_still_has_708_faces_in_all_modes() {
    let mesh = load_obj(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap();
    let verts = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
    for mode in TriangleMode::ALL {
        let mut fb = Framebuffer::new(800, 600);
        let stats = render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &verts,
            Projection::Fit {
                plane: Default::default(),
                margin_ratio: 0.1,
            },
            mode,
        )
        .unwrap();
        assert_eq!(stats.triangles_processed, 708, "{mode}");
        assert!(fb.count_non_background() > 1000, "{mode}");
    }
}

#[test]
fn scene_argument_is_parsed_with_ship_as_default() {
    assert_eq!(args(&[]).unwrap().scene, Scene::Ship);
    assert_eq!(args(&["--scene", "ship"]).unwrap().scene, Scene::Ship);
    assert_eq!(
        args(&["--scene", "triangles", "--mode", "scanline"])
            .unwrap()
            .scene,
        Scene::Triangles
    );
    assert!(args(&["--scene"]).is_err());
    assert!(args(&["--scene", "cube"]).unwrap_err().contains("cube"));
}

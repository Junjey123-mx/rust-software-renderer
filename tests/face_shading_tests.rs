use raylib::math::Vector3;
use rust_software_renderer::geometry::{Vertex, triangle_scene};
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    DirectionalLight, OrbitCamera, OrbitView, Projection, RenderSettings, Uniforms, ViewPreset,
    Viewport, face_normal, intensity, model_bounds, render_interactive, render_with_mode,
    render_with_settings, shade_faces,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::path::Path;

fn light_from_above() -> DirectionalLight {
    DirectionalLight {
        direction: Vector3::new(0.0, 1.0, 0.0),
        ambient: 0.2,
        base_color: Vector3::one(),
    }
}

#[test]
fn normals_and_intensity_follow_the_light() {
    let n = face_normal(
        Vector3::zero(),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
    .unwrap();
    assert!((n.length() - 1.0).abs() < 1e-6);
    assert!(
        (n.y.abs() - 1.0).abs() < 1e-6,
        "horizontal face has a vertical normal"
    );
    let l = light_from_above();
    assert!(
        (intensity(n, &l) - 1.0).abs() < 1e-6,
        "face facing the light is fully lit"
    );
    let side = face_normal(
        Vector3::zero(),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
    .unwrap();
    assert!(
        (intensity(side, &l) - 0.2).abs() < 1e-6,
        "face parallel to the light gets ambient only"
    );
    let flipped = -n;
    assert_eq!(intensity(flipped, &l), intensity(n, &l), "two-sided");
    assert!(face_normal(Vector3::zero(), Vector3::one(), Vector3::one() * 2.0).is_none());
    assert!(
        face_normal(
            Vector3::new(f32::NAN, 0.0, 0.0),
            Vector3::one(),
            Vector3::zero()
        )
        .is_none()
    );
}

#[test]
fn shaded_colors_are_bounded_and_stable_per_face() {
    let mut verts = load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap()
    .expand_vertices(Vertex::DEFAULT_COLOR);
    let light = DirectionalLight::default();
    shade_faces(&mut verts, &light);
    let mut again = verts.clone();
    shade_faces(&mut again, &light);
    assert_eq!(verts, again, "deterministic");
    let mut distinct = std::collections::HashSet::new();
    for tri in verts.chunks_exact(3) {
        assert_eq!(tri[0].color, tri[1].color);
        assert_eq!(tri[1].color, tri[2].color);
        for c in [tri[0].color.x, tri[0].color.y, tri[0].color.z] {
            assert!(c.is_finite() && (0.0..=1.0).contains(&c));
        }
        distinct.insert(format!("{:.3}", tri[0].color.x));
    }
    assert!(
        distinct.len() > 5,
        "faces at different angles get different tones"
    );
    assert!(
        verts.iter().all(|v| v.position == v.transformed_position),
        "geometry untouched"
    );
}

#[test]
fn lit_face_is_brighter_than_a_side_face_in_the_rendered_image() {
    // Both faces project onto the same screen triangle (positions double as
    // screen coordinates); only the depth of the apex changes the normal.
    let light = DirectionalLight {
        direction: Vector3::new(0.0, 0.0, 1.0),
        ambient: 0.2,
        base_color: Vector3::one(),
    };
    let settings = RenderSettings {
        depth_test: true,
        shading: Some(light),
    };
    let brightness = |apex_z: f32| {
        let verts: Vec<Vertex> = [(10.0, 10.0, 0.0), (90.0, 10.0, 0.0), (50.0, 90.0, apex_z)]
            .iter()
            .map(|&(x, y, z)| Vertex::new(Vector3::new(x, y, z)))
            .collect();
        let mut fb = Framebuffer::new(100, 100);
        render_with_settings(
            &mut fb,
            &Uniforms::default(),
            &verts,
            Projection::Screen,
            TriangleMode::FilledBarycentric,
            settings,
        )
        .unwrap();
        fb.get_pixel(50, 40).unwrap().x
    };
    let lit = brightness(0.0); // normal along Z: faces the light
    let dim = brightness(1000.0); // normal almost perpendicular to Z
    assert!((lit - 1.0).abs() < 1e-5, "lit {lit}");
    assert!((0.2..0.35).contains(&dim), "dim {dim}");
}

#[test]
fn wireframe_keeps_vertex_colors_and_fills_match_each_other() {
    let verts = load_obj(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/models/Laboratorio3.obj"
    )))
    .unwrap()
    .expand_vertices(Vertex::DEFAULT_COLOR);
    let b = model_bounds(&verts).unwrap();
    let view = OrbitView::new(
        OrbitCamera::new(b.center, ViewPreset::Oblique),
        &b,
        Viewport::new(800, 600, 0.1),
    )
    .unwrap();
    let mut wire = Framebuffer::new(800, 600);
    render_interactive(
        &mut wire,
        &Uniforms::default(),
        &verts,
        view,
        TriangleMode::Wireframe,
    )
    .unwrap();
    let lit: Vec<Vector3> = (0..600)
        .flat_map(|y| (0..800).map(move |x| (x, y)))
        .filter_map(|(x, y)| wire.get_pixel(x, y))
        .filter(|c| *c != Vector3::zero())
        .collect();
    assert!(
        lit.iter().all(|c| *c == Vertex::DEFAULT_COLOR),
        "wireframe edges stay white"
    );
    let mut bary = Framebuffer::new(800, 600);
    let mut scan = Framebuffer::new(800, 600);
    render_interactive(
        &mut bary,
        &Uniforms::default(),
        &verts,
        view,
        TriangleMode::FilledBarycentric,
    )
    .unwrap();
    render_interactive(
        &mut scan,
        &Uniforms::default(),
        &verts,
        view,
        TriangleMode::FilledScanline,
    )
    .unwrap();
    assert_eq!(bary.to_rgba_bytes(), scan.to_rgba_bytes());
    let tones: std::collections::HashSet<[u8; 3]> = bary
        .to_rgba_bytes()
        .chunks(4)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    assert!(
        tones.len() > 8,
        "shaded ship shows several tones, got {}",
        tones.len()
    );
}

#[test]
fn synthetic_scene_colors_are_untouched_by_shading_defaults() {
    let mut fb = Framebuffer::new(800, 600);
    render_with_mode(
        &mut fb,
        &Uniforms::default(),
        &triangle_scene(),
        Projection::Screen,
        TriangleMode::FilledScanline,
    )
    .unwrap();
    assert_eq!(fb.get_pixel(330, 220), Some(Vector3::new(1.0, 0.75, 0.2)));
}

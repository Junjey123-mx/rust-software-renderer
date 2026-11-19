//! The ring of diamond playing cards around House of Cards.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::pipeline::{
    NormalSource, OrbitCamera, OrbitView, ShadedOptions, Transform, Uniforms, ViewPreset,
    ViewProjection, Viewport, render_shaded_with,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::card_ring::{
    BORDER, CARD_HEIGHT, CARD_THICKNESS, CARD_WIDTH, CardShader, CardSide, card_mesh, card_pattern,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

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

fn faces(mesh: &Mesh) -> Vec<(Vector3, Vector3)> {
    mesh.indices
        .chunks(3)
        .map(|t| {
            let (a, b, c) = (
                mesh.vertices[t[0]],
                mesh.vertices[t[1]],
                mesh.vertices[t[2]],
            );
            ((b - a).cross(c - a), (a + b + c) / 3.0)
        })
        .collect()
}

#[test]
fn the_card_is_a_thin_closed_slab_of_playing_card_proportions() {
    let card = card_mesh();
    assert!(is_closed(&card));
    assert_eq!(card.triangle_count(), 28);
    let (mut lo, mut hi) = (Vector3::one() * 9.0, Vector3::one() * -9.0);
    for p in &card.vertices {
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        lo = Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
        hi = Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
    }
    let size = hi - lo;
    assert!((size.x - CARD_WIDTH).abs() < 1e-6 && (size.y - CARD_HEIGHT).abs() < 1e-6);
    assert!((size.z - CARD_THICKNESS).abs() < 1e-6 && CARD_THICKNESS < 0.05 * CARD_HEIGHT);
    assert!((CARD_HEIGHT / CARD_WIDTH - 3.5 / 2.5).abs() < 0.01);
}

#[test]
fn every_face_points_outwards_and_belongs_to_the_face_the_back_or_the_edge() {
    let mut count: HashMap<&str, usize> = HashMap::new();
    for (normal, centroid) in faces(&card_mesh()) {
        assert!(
            normal.length() > 1e-7 && normal.dot(centroid) > 0.0,
            "outward"
        );
        let side = match CardSide::of(normal) {
            CardSide::Face => "face",
            CardSide::Back => "back",
            CardSide::Edge => "edge",
        };
        *count.entry(side).or_default() += 1;
        if side == "edge" {
            assert!(
                normal.normalized().z.abs() < 1e-5,
                "edges are perpendicular"
            );
        }
    }
    assert_eq!((count["face"], count["back"], count["edge"]), (6, 6, 16));
}

fn is_neon(c: Vector3) -> bool {
    c.x > 2.0 * c.y && c.z > 1.5 * c.y && c.x > 0.4
}

#[test]
fn the_face_shows_a_neon_diamond_with_pips_inside_a_corinto_border() {
    let (center, glow) = card_pattern(CardSide::Face, 0.0, 0.0);
    assert!(is_neon(center) && glow > 0.3, "{center:?}");
    let w = CARD_WIDTH / 2.0;
    let h = CARD_HEIGHT / 2.0;
    let (pip, _) = card_pattern(CardSide::Face, -0.62 * w, 0.74 * h);
    assert!(is_neon(pip), "corner pip {pip:?}");
    let (border, _) = card_pattern(CardSide::Face, 0.95 * w, 0.0);
    assert_eq!(border, BORDER);
    let (ground, glow) = card_pattern(CardSide::Face, 0.7 * w, 0.1 * h);
    assert!(
        ground.length() < 0.35 && glow == 0.0,
        "dark ground {ground:?}"
    );
    // The diamond is a rhombus: inside along the axes, outside on the diagonal.
    assert!(is_neon(card_pattern(CardSide::Face, 0.0, 0.55 * h).0));
    assert!(!is_neon(card_pattern(CardSide::Face, 0.35 * w, 0.42 * h).0));
    // The back is a different design, the edge is plain.
    assert_ne!(
        card_pattern(CardSide::Back, 0.0, 0.3 * h),
        card_pattern(CardSide::Face, 0.0, 0.3 * h)
    );
    assert_eq!(card_pattern(CardSide::Edge, 0.0, 0.0).1, 0.0);
}

#[test]
fn the_diamond_glows_even_when_the_card_turns_away_from_the_star() {
    let lit = CardShader {
        light: Vector3::new(0.0, 0.0, 1.0),
    };
    let dark = CardShader {
        light: Vector3::new(0.0, 0.0, -1.0),
    };
    let n = Vector3::new(0.0, 0.0, 1.0);
    let (a, b) = (
        lit.shade(Vector3::zero(), n),
        dark.shade(Vector3::zero(), n),
    );
    assert!(a.x > b.x && is_neon(b), "{a:?} {b:?}");
}

/// Draws the card under the orthographic front camera with back faces
/// culled and counts the shaded samples of each side.
fn draw(yaw: f32) -> (Framebuffer, [usize; 3]) {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Front),
        0.6,
        Viewport::new(160, 160, 0.1),
    )
    .unwrap();
    let transform = Transform {
        yaw,
        ..Transform::default()
    };
    let mesh = card_mesh().expand_vertices(Vector3::one());
    let counts = [
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ];
    let shader = CardShader {
        light: Vector3::new(0.0, 0.0, 1.0),
    };
    let mut fb = Framebuffer::new(160, 160);
    fb.clear();
    render_shaded_with(
        &mut fb,
        &Uniforms {
            model_matrix: transform.matrix(),
        },
        &mesh,
        &ViewProjection::Orbit(view),
        TriangleMode::FilledScanline,
        ShadedOptions {
            cull_back_faces: true,
            normal_source: NormalSource::Face,
            threads: 1,
        },
        &|s, _| {
            counts[CardSide::of(s.face_normal) as usize].fetch_add(1, Ordering::Relaxed);
            shader.shade(s.object_position, s.face_normal)
        },
    )
    .unwrap();
    (fb, counts.map(|c| c.into_inner()))
}

#[test]
fn culling_and_depth_show_only_the_side_that_faces_the_camera() {
    // Turn the card's face towards the camera's eye.
    let eye = OrbitCamera::new(Vector3::zero(), ViewPreset::Front).eye_direction();
    let facing = if eye.z > 0.0 {
        0.0
    } else {
        std::f32::consts::PI
    };
    let (front, seen) = draw(facing);
    assert!(seen[CardSide::Face as usize] > 1000, "{seen:?}");
    assert_eq!(seen[CardSide::Back as usize], 0);
    assert!(
        is_neon(front.get_pixel(80, 80).unwrap()),
        "the diamond at the center: {:?}",
        front.get_pixel(80, 80)
    );
    let (back, seen) = draw(facing + std::f32::consts::PI);
    assert!(seen[CardSide::Back as usize] > 1000 && seen[CardSide::Face as usize] == 0);
    assert_ne!(back.get_pixel(80, 80), front.get_pixel(80, 80));
    // Edge-on, only the thin edge is left.
    let (_, edge_on) = draw(facing + std::f32::consts::FRAC_PI_2);
    assert!(edge_on[CardSide::Edge as usize] > 0 && edge_on[CardSide::Face as usize] == 0);
    assert_eq!(edge_on[CardSide::Back as usize], 0);
}

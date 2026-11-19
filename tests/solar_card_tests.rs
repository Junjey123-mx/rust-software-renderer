//! The ring of diamond playing cards around House of Cards.

use raylib::math::{Matrix, Vector3};
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::pipeline::{
    NormalSource, OrbitCamera, OrbitView, ShadedOptions, Transform, Uniforms, ViewPreset,
    ViewProjection, Viewport, render_shaded_with,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::SimulationClock;
use rust_software_renderer::solar::card_ring::{
    BORDER, CARD_HEIGHT, CARD_THICKNESS, CARD_WIDTH, CardRing, CardRingSpec, CardShader, CardSide,
    card_mesh, card_pattern,
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

fn apply(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

/// World corners of every card of `ring` around a planet at `center`.
fn corners(ring: &CardRing, center: Vector3, radius: f32) -> Vec<Vec<Vector3>> {
    let mesh = card_mesh();
    (0..ring.slots.len())
        .map(|i| {
            let m = ring.frame(i, center, radius, 0.0, 0.0).matrix();
            mesh.vertices.iter().map(|&p| apply(&m, p)).collect()
        })
        .collect()
}

#[test]
fn the_ring_deals_eighteen_to_twenty_four_cards_that_share_one_mesh() {
    let ring = CardRing::diamonds();
    assert!((18..=24).contains(&ring.slots.len()));
    assert_eq!(ring.mesh.len(), card_mesh().indices.len());
    assert!(std::sync::Arc::ptr_eq(&ring.mesh, &ring.clone().mesh));
}

#[test]
fn the_seed_fixes_every_card_and_another_seed_deals_another_ring() {
    let a = CardRing::diamonds();
    assert_eq!(a.slots, CardRing::diamonds().slots);
    let b = CardRing::new(CardRingSpec {
        seed: 8,
        ..CardRingSpec::default()
    });
    assert_ne!(a.slots, b.slots);
    let distinct = |f: fn(&rust_software_renderer::solar::card_ring::CardSlot) -> f32| {
        let mut v: Vec<f32> = a.slots.iter().map(f).collect();
        v.sort_by(f32::total_cmp);
        v.windows(2).all(|w| w[1] - w[0] > 1e-6)
    };
    assert!(distinct(|s| s.radius) && distinct(|s| s.tilt) && distinct(|s| s.flutter_phase));
}

#[test]
fn each_card_frame_is_a_rotation_with_the_long_side_along_the_radius() {
    let ring = CardRing::diamonds();
    let center = Vector3::new(4.0, 0.0, -3.0);
    for i in 0..ring.slots.len() {
        let f = ring.frame(i, center, 0.78, 0.0, 0.0);
        for (a, b) in [(f.x, f.y), (f.y, f.z), (f.z, f.x)] {
            assert!(a.dot(b).abs() < 1e-5);
        }
        assert!((f.x.cross(f.y) - f.z).length() < 1e-5, "right-handed");
        let out = (f.origin - center).normalized();
        assert!(out.dot(f.y) > 0.99, "card {i} long side off the radius");
        assert!(
            f.z.dot(ring.normal()) > 0.4,
            "card {i} face off the ring plane"
        );
    }
}

#[test]
fn every_card_stays_outside_the_planet_and_inside_the_reserved_ring() {
    let ring = CardRing::diamonds();
    let (center, radius) = (Vector3::new(6.2, 0.0, 0.0), 0.78);
    assert!(ring.extent() <= 2.2, "extent {}", ring.extent());
    for (i, card) in corners(&ring, center, radius).iter().enumerate() {
        for p in card {
            let d = (*p - center).length() / radius;
            assert!(d > 1.3, "card {i} touches the planet ({d})");
            assert!(
                d <= ring.extent() + 1e-4,
                "card {i} beyond the extent ({d})"
            );
        }
    }
}

#[test]
fn neighbouring_cards_keep_a_gap() {
    let ring = CardRing::diamonds();
    let radius = 0.78;
    let frames: Vec<_> = (0..ring.slots.len())
        .map(|i| ring.frame(i, Vector3::zero(), radius, 0.0, 0.0))
        .collect();
    let reach = CARD_WIDTH.hypot(CARD_HEIGHT) / 2.0 * ring.spec.card_size * radius;
    for (i, a) in frames.iter().enumerate() {
        for b in &frames[i + 1..] {
            let gap = (a.origin - b.origin).length();
            assert!(gap > 2.0 * reach, "cards {gap} apart, reach {reach}");
        }
    }
}

#[test]
fn the_ring_is_inclined_and_follows_the_planet_and_its_size() {
    let ring = CardRing::diamonds();
    let tilt = ring.normal().dot(Vector3::new(0.0, 1.0, 0.0)).acos();
    assert!((tilt - ring.spec.inclination).abs() < 1e-5);
    assert!(tilt > 0.2, "the ring lies in the orbital plane");
    let (c, r) = (Vector3::new(-2.0, 1.0, 5.0), 1.7);
    for i in 0..ring.slots.len() {
        let unit = ring.frame(i, Vector3::zero(), 1.0, 0.3, 0.1);
        let moved = ring.frame(i, c, r, 0.3, 0.1);
        assert!((moved.origin - (c + unit.origin * r)).length() < 1e-4);
        assert!((moved.z - unit.z).length() < 1e-6);
        assert!((moved.size - unit.size * r).abs() < 1e-6);
        let off_plane = unit.origin.dot(ring.normal()).abs();
        assert!(off_plane <= ring.spec.thickness + 1e-5);
    }
}

#[test]
fn the_ring_turns_with_simulated_time_and_every_card_rocks_at_its_own_pace() {
    let ring = CardRing::diamonds();
    let (c, r) = (Vector3::new(6.2, 0.0, 0.0), 0.78);
    let n = ring.normal();
    let angle_at = |i: usize, t: f32| {
        let o = (ring.pose(i, c, r, t).origin - c).normalized();
        let a = (ring.pose(i, c, r, 0.0).origin - c).normalized();
        a.cross(o).dot(n).atan2(a.dot(o))
    };
    for i in 0..ring.slots.len() {
        assert!((angle_at(i, 2.0) - 2.0 * ring.spec.orbit_speed).abs() < 1e-3);
    }
    let rock = |i: usize, t: f32| ring.pose(i, c, r, t).z.dot(n).acos();
    let changes: Vec<f32> = (0..ring.slots.len())
        .map(|i| rock(i, 1.0) - rock(i, 0.0))
        .collect();
    assert!(changes.iter().any(|d| d.abs() > 0.01), "cards do not rock");
    assert!(
        changes.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01),
        "every card rocks in step"
    );
}

#[test]
fn a_paused_clock_freezes_the_ring_and_equal_times_give_equal_cards() {
    let ring = CardRing::diamonds();
    let c = Vector3::new(6.2, 0.0, 0.0);
    let mut clock = SimulationClock::new(3.5);
    clock.toggle_pause();
    let before: Vec<_> = (0..ring.slots.len())
        .map(|i| ring.pose(i, c, 0.78, clock.time))
        .collect();
    for _ in 0..10 {
        clock.advance(0.05);
    }
    let after: Vec<_> = (0..ring.slots.len())
        .map(|i| ring.pose(i, c, 0.78, clock.time))
        .collect();
    assert_eq!(before, after);
    let again = CardRing::diamonds();
    assert!((0..ring.slots.len()).all(|i| again.pose(i, c, 0.78, 3.5) == before[i]));
}

#[test]
fn the_cards_stay_valid_and_apart_while_the_ring_turns() {
    let ring = CardRing::diamonds();
    let (c, r) = (Vector3::new(6.2, 0.0, 0.0), 0.78);
    let mesh = card_mesh();
    let reach = CARD_WIDTH.hypot(CARD_HEIGHT) / 2.0 * ring.spec.card_size * r;
    for step in 0..60 {
        let t = step as f32 * 1.7;
        let frames: Vec<_> = (0..ring.slots.len())
            .map(|i| ring.pose(i, c, r, t))
            .collect();
        for (i, f) in frames.iter().enumerate() {
            assert!(f.z.dot(ring.normal()) > 0.4, "card {i} edge-on at {t}");
            let m = f.matrix();
            for &p in &mesh.vertices {
                let d = (apply(&m, p) - c).length() / r;
                assert!(d > 1.3 && d <= ring.extent() + 1e-4, "card {i} at {t}: {d}");
            }
            for g in &frames[i + 1..] {
                assert!((f.origin - g.origin).length() > 2.0 * reach);
            }
        }
    }
}

//! Project 3 light trails (C68): a ribbon behind every neon ship and a
//! wall behind every lightcycle, rebuilt from the route alone, of constant
//! world length, 32 to 48 triangles each, frozen by the pause and fading
//! to the tail.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::path::Path;
use rust_software_renderer::solar::attach::traffic::{
    CYCLE_TRAIL, SHIP_TRAIL, TrailKind, TrailShader, TrailSpec, build_trail,
};
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

fn field() -> HeightField {
    HeightField::Hearts(HeartsField::new(11))
}

fn routes() -> [(Path, TrailSpec); 2] {
    [
        (
            Path::GreatCircle {
                axis: Vector3::new(0.2, 1.0, 0.1),
                speed: 0.25,
                phase: 0.3,
                altitude: 0.45,
            },
            SHIP_TRAIL,
        ),
        (
            Path::Ring {
                axis: Vector3::new(1.0, 0.1, -0.3),
                radius: 0.1,
                speed: -1.8,
                phase: 2.0,
                altitude: 0.01,
            },
            CYCLE_TRAIL,
        ),
    ]
}

fn trail(path: &Path, t: f32, spec: &TrailSpec) -> Vec<Vertex> {
    let (mut poses, mut out) = (Vec::new(), Vec::new());
    build_trail(path, t, spec, &field(), &mut poses, &mut out).unwrap();
    out
}

/// Length of the trail's spine (the middle of each cross edge).
fn spine_length(v: &[Vertex]) -> f32 {
    // Each segment is [a, c, d, a, d, b]: a/b at its tail end, c/d ahead.
    let mids: Vec<Vector3> = v
        .chunks(6)
        .map(|q| (q[0].position + q[5].position) * 0.5)
        .chain(
            v.chunks(6)
                .last()
                .map(|q| (q[1].position + q[2].position) * 0.5),
        )
        .collect();
    mids.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap()
}

#[test]
fn world_length_constant_within_tolerance() {
    for (path, spec) in routes() {
        let lengths: Vec<f32> = (0..40)
            .map(|k| spine_length(&trail(&path, k as f32 * 0.37, &spec)))
            .collect();
        let (lo, hi) = lengths
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &l| (a.min(l), b.max(l)));
        assert!(lo > 0.0 && hi / lo < 1.1, "{path:?}: {lo}..{hi}");
    }
}

#[test]
fn triangles_32_to_48() {
    for (path, spec) in routes() {
        for segs in [16, 20, 24] {
            let v = trail(&path, 3.0, &TrailSpec { segs, ..spec });
            assert_eq!(v.len() / 3, 2 * segs);
        }
    }
    // Negative cases: out of range is an error, no length is no trail.
    let (path, spec) = routes()[0];
    let (mut poses, mut out) = (
        Vec::new(),
        vec![Vertex::new_with_color(Vector3::zero(), Vector3::one())],
    );
    for segs in [15, 25] {
        let bad = TrailSpec { segs, ..spec };
        assert!(build_trail(&path, 1.0, &bad, &field(), &mut poses, &mut out).is_err());
    }
    let none = TrailSpec { len_s: 0.0, ..spec };
    build_trail(&path, 1.0, &none, &field(), &mut poses, &mut out).unwrap();
    assert!(out.is_empty());
}

#[test]
fn no_history_needed() {
    for (path, spec) in routes() {
        // The same time gives the same trail, whatever came before, and the
        // buffers are reused.
        let fresh = trail(&path, 5.0, &spec);
        let (mut poses, mut out) = (Vec::new(), Vec::new());
        for t in [9.0, 1.0, 5.0] {
            build_trail(&path, t, &spec, &field(), &mut poses, &mut out).unwrap();
        }
        assert_eq!(out, fresh);
        let capacity = (poses.capacity(), out.capacity());
        build_trail(&path, 7.0, &spec, &field(), &mut poses, &mut out).unwrap();
        assert_eq!((poses.capacity(), out.capacity()), capacity);
    }
}

#[test]
fn paused_frozen() {
    let mut s = scene();
    s.clock.time = 4.0;
    s.update();
    let traffic = s.traffic.as_ref().unwrap();
    assert_eq!(
        traffic.trails.len(),
        traffic.ships.len() + traffic.cycles.len()
    );
    assert!(traffic.trails.iter().all(|t| !t.is_empty()));
    let before = traffic.trails.clone();
    s.clock.paused = true;
    for _ in 0..60 {
        s.clock.advance(1.0 / 60.0);
        s.update();
    }
    assert_eq!(s.traffic.as_ref().unwrap().trails, before);
    s.clock.paused = false;
    s.clock.advance(1.0 / 60.0);
    s.update();
    assert_ne!(s.traffic.as_ref().unwrap().trails, before);
}

#[test]
fn tapers_to_tail() {
    for (path, spec) in routes() {
        let v = trail(&path, 6.0, &spec);
        // Size of the first (tail) and last (head) cross edges.
        let tail = (v[0].position - v[5].position).length();
        let last = v.chunks(6).last().unwrap();
        let head = (last[1].position - last[2].position).length();
        assert!(tail < 1e-5 && head > 0.0, "{tail} {head}");
        if let TrailKind::Wall { .. } = spec.kind {
            // The wall stands up from the route.
            let up = (last[2].position - last[1].position).normalized();
            assert!(up.dot(last[1].position.normalized()) > 0.99);
        }
        // Brighter at the head than at the tail.
        let shader = TrailShader {
            color: Vector3::new(0.2, 0.9, 1.0),
            head: last[1].position,
            tail: v[0].position,
        };
        let (h, t) = (shader.shade(last[1].position), shader.shade(v[0].position));
        assert!(h.y > 3.0 * t.y, "{h:?} {t:?}");
    }
}

//! Coordinate-level parity between the two independent fill algorithms on
//! the instructor-inspired matrix (T01-T09). Failures list exact pixels.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{RasterBounds, fill_barycentric, fill_scanline};
use rust_software_renderer::rendering::Fragment;
use std::collections::HashSet;

type P = (f32, f32);
type Coords = Vec<(i32, i32)>;

const CASES: [(&str, P, P, P); 9] = [
    ("T01 small", (10.0, 10.0), (18.0, 12.0), (12.0, 20.0)),
    ("T02 large", (10.0, 10.0), (90.0, 20.0), (40.0, 85.0)),
    (
        "T03 horizontal edge",
        (10.0, 20.0),
        (70.0, 20.0),
        (40.0, 75.0),
    ),
    (
        "T04 vertical edge",
        (20.0, 10.0),
        (20.0, 80.0),
        (75.0, 40.0),
    ),
    (
        "T05 reversed winding",
        (10.0, 10.0),
        (40.0, 85.0),
        (90.0, 20.0),
    ),
    (
        "T06 partial clip",
        (-15.0, 20.0),
        (50.0, 15.0),
        (40.0, 115.0),
    ),
    ("T07 collinear", (10.0, 10.0), (30.0, 30.0), (50.0, 50.0)),
    (
        "T08 repeated vertex",
        (20.0, 20.0),
        (20.0, 20.0),
        (60.0, 50.0),
    ),
    (
        "T09 samples on edges",
        (10.5, 10.5),
        (60.5, 10.5),
        (10.5, 60.5),
    ),
];

const B: RasterBounds = RasterBounds {
    width: 100,
    height: 100,
};

fn v(p: P) -> Vertex {
    Vertex::new(Vector3::new(p.0, p.1, 0.0))
}

fn coords(frags: &[Fragment]) -> Vec<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

fn both(a: P, b: P, c: P) -> (Coords, Coords) {
    (
        coords(&fill_barycentric(&v(a), &v(b), &v(c), B)),
        coords(&fill_scanline(&v(a), &v(b), &v(c), B)),
    )
}

/// Asserts no duplicates per algorithm and identical coordinate sets,
/// printing the first differing pixels on failure.
fn assert_parity(name: &str, bary: &[(i32, i32)], scan: &[(i32, i32)]) {
    let bs: HashSet<_> = bary.iter().copied().collect();
    let ss: HashSet<_> = scan.iter().copied().collect();
    assert_eq!(
        bs.len(),
        bary.len(),
        "{name}: barycentric emitted duplicate pixels"
    );
    assert_eq!(
        ss.len(),
        scan.len(),
        "{name}: scanline emitted duplicate pixels"
    );
    let mut only_b: Vec<_> = bs.difference(&ss).copied().collect();
    let mut only_s: Vec<_> = ss.difference(&bs).copied().collect();
    only_b.sort();
    only_s.sort();
    assert!(
        only_b.is_empty() && only_s.is_empty(),
        "{name}: coverage differs (bary {} px, scan {} px)\n only_in_barycentric (first 20): {:?}\n only_in_scanline (first 20): {:?}",
        bs.len(),
        ss.len(),
        &only_b[..only_b.len().min(20)],
        &only_s[..only_s.len().min(20)]
    );
    for (x, y) in &bs {
        assert!(
            (0..100).contains(x) && (0..100).contains(y),
            "{name}: ({x},{y}) outside"
        );
    }
}

#[test]
fn matrix_t01_to_t09_has_identical_coverage() {
    for (name, a, b, c) in CASES {
        let (bary, scan) = both(a, b, c);
        assert_parity(name, &bary, &scan);
    }
}

#[test]
fn non_degenerate_cases_produce_interior_and_degenerate_none() {
    for (name, a, b, c) in CASES {
        let (bary, scan) = both(a, b, c);
        if name.starts_with("T07") || name.starts_with("T08") {
            assert!(bary.is_empty() && scan.is_empty(), "{name}");
        } else {
            assert!(
                bary.len() > 10 && scan.len() > 10,
                "{name}: {} / {}",
                bary.len(),
                scan.len()
            );
        }
    }
}

#[test]
fn reversing_vertices_keeps_both_algorithms_unchanged() {
    for (name, a, b, c) in CASES {
        let (b1, s1) = both(a, b, c);
        let (b2, s2) = both(a, c, b);
        let (b3, s3) = both(c, b, a);
        let set = |v: &[(i32, i32)]| v.iter().copied().collect::<HashSet<_>>();
        assert_eq!(set(&b1), set(&b2), "{name} barycentric swap");
        assert_eq!(set(&b1), set(&b3), "{name} barycentric rotate");
        assert_eq!(set(&s1), set(&s2), "{name} scanline swap");
        assert_eq!(set(&s1), set(&s3), "{name} scanline rotate");
    }
}

#[test]
fn t05_reversed_winding_matches_t02() {
    let (b2, _) = both((10.0, 10.0), (90.0, 20.0), (40.0, 85.0));
    let (b5, s5) = both((10.0, 10.0), (40.0, 85.0), (90.0, 20.0));
    let set = |v: &[(i32, i32)]| v.iter().copied().collect::<HashSet<_>>();
    assert_eq!(set(&b2), set(&b5));
    assert_eq!(set(&b2), set(&s5));
}

#[test]
fn adjacent_triangles_share_edges_without_gaps_or_overlap() {
    // Square split along its diagonal, plus a square split by a horizontal
    // and a vertical cut: each pixel center of the square is owned by exactly
    // one triangle, in both algorithms.
    let pairs: [[(P, P, P); 2]; 3] = [
        [
            ((10.0, 10.0), (60.0, 10.0), (60.0, 60.0)),
            ((10.0, 10.0), (60.0, 60.0), (10.0, 60.0)),
        ],
        [
            ((10.0, 10.0), (60.0, 10.0), (35.0, 35.0)),
            ((10.0, 60.0), (35.0, 35.0), (60.0, 60.0)),
        ],
        [
            ((10.0, 10.0), (35.5, 35.5), (10.0, 60.0)),
            ((60.0, 10.0), (60.0, 60.0), (35.5, 35.5)),
        ],
    ];
    for pair in pairs {
        for algo in 0..2 {
            let run = |t: (P, P, P)| -> Vec<(i32, i32)> {
                if algo == 0 {
                    coords(&fill_barycentric(&v(t.0), &v(t.1), &v(t.2), B))
                } else {
                    coords(&fill_scanline(&v(t.0), &v(t.1), &v(t.2), B))
                }
            };
            let a: HashSet<_> = run(pair[0]).into_iter().collect();
            let b: HashSet<_> = run(pair[1]).into_iter().collect();
            let overlap: Vec<_> = a.intersection(&b).collect();
            assert!(overlap.is_empty(), "algo {algo}: shared pixels {overlap:?}");
            // Diagonal split covers the whole 50x50 square of centers.
            if pair[0].2 == (60.0, 60.0) {
                assert_eq!(
                    a.len() + b.len(),
                    50 * 50,
                    "algo {algo}: gaps on the shared edge"
                );
            }
        }
    }
}

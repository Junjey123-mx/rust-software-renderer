//! Deterministic stress corpus: seeded pseudo-random triangles, all vertex
//! permutations, extreme clipping and subpixel alignment. Both fill
//! algorithms must agree pixel for pixel on every single triangle.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::rasterization::{RasterBounds, fill_barycentric, fill_scanline};
use rust_software_renderer::rendering::Fragment;
use std::collections::HashSet;

type P = (f32, f32);

/// Minimal LCG (Numerical Recipes constants); no external crate.
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    /// Uniform in `[lo, hi)`, quantised to quarter pixels so `.0`, `.25`,
    /// `.5` and `.75` alignments (ties on centers included) all appear.
    fn coord(&mut self, lo: f32, hi: f32) -> f32 {
        let steps = ((hi - lo) * 4.0) as u32;
        lo + (self.next_u32() % steps) as f32 / 4.0
    }
}

fn v(p: P) -> Vertex {
    Vertex::new(Vector3::new(p.0, p.1, 0.0))
}

fn coords(frags: &[Fragment]) -> Vec<(i32, i32)> {
    frags
        .iter()
        .map(|f| (f.position.x as i32, f.position.y as i32))
        .collect()
}

fn check(label: &str, a: P, b: P, c: P, bounds: RasterBounds) -> usize {
    let bary = coords(&fill_barycentric(&v(a), &v(b), &v(c), bounds));
    let scan = coords(&fill_scanline(&v(a), &v(b), &v(c), bounds));
    let bs: HashSet<_> = bary.iter().copied().collect();
    let ss: HashSet<_> = scan.iter().copied().collect();
    assert_eq!(
        bs.len(),
        bary.len(),
        "{label} {a:?} {b:?} {c:?}: barycentric duplicates"
    );
    assert_eq!(
        ss.len(),
        scan.len(),
        "{label} {a:?} {b:?} {c:?}: scanline duplicates"
    );
    let mut only_b: Vec<_> = bs.difference(&ss).copied().collect();
    let mut only_s: Vec<_> = ss.difference(&bs).copied().collect();
    only_b.sort();
    only_s.sort();
    assert!(
        only_b.is_empty() && only_s.is_empty(),
        "{label} {a:?} {b:?} {c:?} bounds {bounds:?}: only_bary={:?} only_scan={:?}",
        &only_b[..only_b.len().min(10)],
        &only_s[..only_s.len().min(10)]
    );
    for (x, y) in &bs {
        assert!(
            (0..bounds.width as i32).contains(x) && (0..bounds.height as i32).contains(y),
            "{label}: ({x},{y}) outside {bounds:?}"
        );
    }
    bs.len()
}

#[test]
fn seeded_random_triangles_agree_in_every_viewport() {
    let mut rng = Lcg(0x5EED_1AB5);
    let viewports = [(100, 100), (64, 48), (320, 200), (7, 5)];
    let mut covered_total = 0;
    for i in 0..300 {
        let (w, h) = viewports[i % viewports.len()];
        let (lo, hi) = (-(w as f32) * 0.5, (w as f32) * 1.5);
        let (lo_y, hi_y) = (-(h as f32) * 0.5, (h as f32) * 1.5);
        let a = (rng.coord(lo, hi), rng.coord(lo_y, hi_y));
        let b = (rng.coord(lo, hi), rng.coord(lo_y, hi_y));
        let c = (rng.coord(lo, hi), rng.coord(lo_y, hi_y));
        covered_total += check(
            &format!("seed triangle {i}"),
            a,
            b,
            c,
            RasterBounds::new(w, h),
        );
    }
    assert!(covered_total > 10_000, "corpus too sparse: {covered_total}");
}

#[test]
fn all_six_permutations_agree() {
    let tris: [(P, P, P); 4] = [
        ((10.0, 10.0), (90.0, 20.0), (40.0, 85.0)),
        ((10.5, 10.5), (60.5, 10.5), (10.5, 60.5)),
        ((3.25, 7.75), (40.0, 7.75), (20.5, 50.25)),
        ((0.0, 0.0), (99.0, 99.0), (99.0, 0.0)),
    ];
    for (a, b, c) in tris {
        let base = check("perm abc", a, b, c, RasterBounds::new(100, 100));
        for (p, q, r) in [(a, c, b), (b, a, c), (b, c, a), (c, a, b), (c, b, a)] {
            assert_eq!(check("perm", p, q, r, RasterBounds::new(100, 100)), base);
        }
    }
}

#[test]
fn tiny_but_non_degenerate_triangles() {
    for (a, b, c) in [
        ((10.0, 10.0), (10.75, 10.0), (10.0, 10.75)),
        ((5.5, 5.5), (6.5, 5.5), (5.5, 6.5)),
        ((20.25, 20.25), (20.5, 20.25), (20.25, 20.5)),
        ((0.0, 0.0), (0.0, 1.0), (1.0, 0.0)),
    ] {
        check("tiny", a, b, c, RasterBounds::new(50, 50));
    }
}

#[test]
fn clipping_across_every_border_and_fully_outside() {
    let b = RasterBounds::new(100, 100);
    assert!(check("left", (-80.0, 20.0), (30.0, 10.0), (20.0, 90.0), b) > 0);
    assert!(check("right", (70.0, 10.0), (180.0, 50.0), (70.0, 90.0), b) > 0);
    assert!(check("top", (20.0, -70.0), (80.0, -60.0), (50.0, 40.0), b) > 0);
    assert!(check("bottom", (20.0, 60.0), (80.0, 70.0), (50.0, 170.0), b) > 0);
    assert_eq!(
        check(
            "all four",
            (-300.0, 50.0),
            (350.0, -300.0),
            (350.0, 400.0),
            b
        ),
        100 * 100
    );
    assert_eq!(
        check("outside", (200.0, 200.0), (300.0, 200.0), (200.0, 300.0), b),
        0
    );
    assert_eq!(
        check(
            "outside neg",
            (-50.0, -50.0),
            (-10.0, -50.0),
            (-50.0, -10.0),
            b
        ),
        0
    );
    assert_eq!(
        check("huge", (-1e7, -1e7), (1e7, -1e7), (0.0, 1e7), b),
        100 * 100
    );
}

#[test]
fn degenerate_inputs_and_empty_viewports() {
    let b = RasterBounds::new(100, 100);
    assert_eq!(
        check("collinear", (10.0, 10.0), (30.0, 30.0), (50.0, 50.0), b),
        0
    );
    assert_eq!(
        check("collinear h", (10.0, 10.0), (30.0, 10.0), (50.0, 10.0), b),
        0
    );
    assert_eq!(
        check("repeat", (20.0, 20.0), (20.0, 20.0), (60.0, 50.0), b),
        0
    );
    assert_eq!(
        check("all same", (20.0, 20.0), (20.0, 20.0), (20.0, 20.0), b),
        0
    );
    for (w, h) in [(0, 0), (0, 10), (10, 0)] {
        assert_eq!(
            check(
                "empty vp",
                (1.0, 1.0),
                (5.0, 1.0),
                (1.0, 5.0),
                RasterBounds::new(w, h)
            ),
            0
        );
    }
    assert_eq!(check("nan", (f32::NAN, 1.0), (5.0, 1.0), (1.0, 5.0), b), 0);
    assert_eq!(
        check("inf", (1.0, 1.0), (f32::INFINITY, 1.0), (1.0, 5.0), b),
        0
    );
}

#[test]
fn seeded_quarter_pixel_triangles_in_a_single_pixel_viewport() {
    // Exercises center ties on a 1x1 framebuffer: the only center is (0.5,0.5).
    let mut rng = Lcg(42);
    for i in 0..200 {
        let a = (rng.coord(-2.0, 3.0), rng.coord(-2.0, 3.0));
        let b = (rng.coord(-2.0, 3.0), rng.coord(-2.0, 3.0));
        let c = (rng.coord(-2.0, 3.0), rng.coord(-2.0, 3.0));
        let n = check(&format!("1x1 #{i}"), a, b, c, RasterBounds::new(1, 1));
        assert!(n <= 1);
    }
}

//! Fixed triangle workload, timing rounds, medians and CSV export.
//!
//! Timed region: the repeated calls to `rasterize_with_mode` only, which
//! include creating and storing every `Fragment` in the returned `Vec`.
//! Workload construction, parity checks, printing and file output happen
//! outside the timer. `black_box` keeps the compiler from eliding the work.

use crate::geometry::{Vertex, triangle_scene};
use crate::pipeline::rasterize_with_mode;
use crate::rasterization::{RasterBounds, TriangleMode, coverage};
use raylib::math::Vector3;
use std::collections::HashSet;
use std::hint::black_box;
use std::time::Instant;

/// The same triangles for every algorithm, already in screen space.
#[derive(Debug, Clone)]
pub struct Workload {
    pub triangles: Vec<[Vertex; 3]>,
    pub bounds: RasterBounds,
}

/// Deterministic workload: the demonstration scene plus 200 seeded triangles
/// (small to large, some partially outside) in an 800x600 viewport.
/// Degenerate triangles are filtered out so every entry produces interior.
pub fn build_workload() -> Workload {
    let bounds = RasterBounds::new(800, 600);
    let mut triangles: Vec<[Vertex; 3]> = triangle_scene()
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect();

    let mut state: u64 = 0x1AB5_0005;
    let mut next = |lo: f32, hi: f32| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let unit = ((state >> 33) as u32) as f32 / u32::MAX as f32;
        lo + (hi - lo) * unit
    };
    let white = Vector3::one();
    while triangles.len() < 207 {
        // Anchor plus two offsets whose magnitude grows with the index, so
        // sizes range from a few pixels to a few hundred.
        let size = 8.0 + (triangles.len() as f32 % 40.0) * 12.0;
        let ax = next(-50.0, 850.0);
        let ay = next(-50.0, 650.0);
        let corners = [
            (ax, ay),
            (ax + next(-size, size), ay + next(-size, size)),
            (ax + next(-size, size), ay + next(-size, size)),
        ];
        let pts = corners.map(|(x, y)| (x as f64, y as f64));
        // Keep only non-degenerate triangles whose bounding box touches the
        // viewport (pure geometry; the rasterizer is not consulted here).
        let Some((_, area2)) = coverage::orient(pts[0], pts[1], pts[2]) else {
            continue;
        };
        let (min_x, max_x) = (
            pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
            pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
        );
        let (min_y, max_y) = (
            pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        );
        if area2 < 2.0 || max_x < 0.0 || max_y < 0.0 || min_x > 800.0 || min_y > 600.0 {
            continue;
        }
        triangles
            .push(corners.map(|(x, y)| Vertex::new_with_color(Vector3::new(x, y, 0.0), white)));
    }
    Workload { triangles, bounds }
}

/// One timed round of `iterations` full-workload rasterizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Round {
    pub mode: TriangleMode,
    pub round: usize,
    pub iterations: usize,
    pub elapsed_ns: u128,
    pub fragments_per_iteration: usize,
}

impl Round {
    pub fn ns_per_iteration(&self) -> f64 {
        self.elapsed_ns as f64 / self.iterations as f64
    }
}

/// Times `iterations` calls of the real rasterizer on the workload.
pub fn time_mode(
    workload: &Workload,
    mode: TriangleMode,
    iterations: usize,
    round: usize,
) -> Round {
    let triangles = black_box(&workload.triangles);
    let bounds = black_box(workload.bounds);
    let mut fragments_per_iteration = 0;

    let start = Instant::now();
    for _ in 0..iterations {
        let fragments = rasterize_with_mode(triangles, mode, bounds);
        fragments_per_iteration = fragments.len();
        black_box(fragments);
    }
    let elapsed_ns = start.elapsed().as_nanos();

    Round {
        mode,
        round,
        iterations,
        elapsed_ns,
        fragments_per_iteration,
    }
}

/// Confirms both fills cover identical pixel sets per triangle on the
/// workload. Returns the fragment count per iteration, or the first
/// mismatch. Performance numbers are meaningless without this.
pub fn verify_parity(workload: &Workload) -> Result<usize, String> {
    let mut total = 0;
    for (i, tri) in workload.triangles.iter().enumerate() {
        let b = rasterize_with_mode(
            std::slice::from_ref(tri),
            TriangleMode::FilledBarycentric,
            workload.bounds,
        );
        let s = rasterize_with_mode(
            std::slice::from_ref(tri),
            TriangleMode::FilledScanline,
            workload.bounds,
        );
        let bs: HashSet<(i32, i32)> = b
            .iter()
            .map(|f| (f.position.x as i32, f.position.y as i32))
            .collect();
        let ss: HashSet<(i32, i32)> = s
            .iter()
            .map(|f| (f.position.x as i32, f.position.y as i32))
            .collect();
        if bs.len() != b.len() || ss.len() != s.len() || bs != ss {
            return Err(format!(
                "triangle {i}: barycentric {} px, scanline {} px, differ",
                bs.len(),
                ss.len()
            ));
        }
        total += b.len();
    }
    Ok(total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkConfig {
    pub warmup_rounds: usize,
    pub rounds: usize,
    pub iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_rounds: 10,
            rounds: 7,
            iterations: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub config: BenchmarkConfig,
    pub triangles: usize,
    pub fragments_per_iteration: usize,
    pub rounds: Vec<Round>,
}

impl BenchmarkReport {
    pub fn rounds_for(&self, mode: TriangleMode) -> Vec<&Round> {
        self.rounds.iter().filter(|r| r.mode == mode).collect()
    }

    /// Median of the per-round durations for `mode`, in nanoseconds.
    pub fn median_round_ns(&self, mode: TriangleMode) -> f64 {
        median_ns(
            &self
                .rounds_for(mode)
                .iter()
                .map(|r| r.elapsed_ns)
                .collect::<Vec<_>>(),
        )
    }
}

/// Median of a sorted copy; even counts use the mean of the two middle values.
pub fn median_ns(values: &[u128]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2] as f64
    } else {
        (v[n / 2 - 1] as f64 + v[n / 2] as f64) / 2.0
    }
}

/// Runs warm-up rounds (discarded) then `rounds` measured rounds per filled
/// mode, alternating modes each round to spread thermal drift evenly.
pub fn run_benchmark(
    workload: &Workload,
    config: BenchmarkConfig,
) -> Result<BenchmarkReport, String> {
    let fragments_per_iteration = verify_parity(workload)?;
    let modes = [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ];
    for _ in 0..config.warmup_rounds {
        for mode in modes {
            black_box(time_mode(
                workload,
                mode,
                config.iterations.max(1) / 10 + 1,
                0,
            ));
        }
    }
    let mut rounds = Vec::with_capacity(config.rounds * modes.len());
    for round in 1..=config.rounds {
        for mode in modes {
            let r = time_mode(workload, mode, config.iterations, round);
            if r.fragments_per_iteration != fragments_per_iteration {
                return Err(format!(
                    "{mode} produced {} fragments, expected {fragments_per_iteration}",
                    r.fragments_per_iteration
                ));
            }
            rounds.push(r);
        }
    }
    Ok(BenchmarkReport {
        config,
        triangles: workload.triangles.len(),
        fragments_per_iteration,
        rounds,
    })
}

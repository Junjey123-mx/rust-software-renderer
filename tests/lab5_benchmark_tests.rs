use rust_software_renderer::app::parse_args;
use rust_software_renderer::benchmark::{
    BenchmarkConfig, build_workload, median_ns, run_benchmark, time_mode, verify_parity,
};
use rust_software_renderer::pipeline::rasterize_with_mode;
use rust_software_renderer::rasterization::TriangleMode;

#[test]
fn workload_is_fixed_and_non_degenerate() {
    let a = build_workload();
    let b = build_workload();
    assert_eq!(a.triangles.len(), 207);
    assert_eq!(a.triangles, b.triangles, "deterministic");
    assert_eq!((a.bounds.width, a.bounds.height), (800, 600));
    for (i, t) in a.triangles.iter().enumerate() {
        let n = rasterize_with_mode(
            std::slice::from_ref(t),
            TriangleMode::FilledBarycentric,
            a.bounds,
        )
        .len();
        assert!(n > 0, "triangle {i} produces no interior");
    }
}

#[test]
fn both_fills_cover_the_workload_identically() {
    let w = build_workload();
    let total = verify_parity(&w).unwrap();
    let bary = rasterize_with_mode(&w.triangles, TriangleMode::FilledBarycentric, w.bounds).len();
    let scan = rasterize_with_mode(&w.triangles, TriangleMode::FilledScanline, w.bounds).len();
    assert_eq!(total, bary);
    assert_eq!(total, scan);
    assert!(total > 100_000, "{total}");
}

#[test]
fn time_mode_counts_real_fragments_and_elapses() {
    let w = build_workload();
    let expected = rasterize_with_mode(&w.triangles, TriangleMode::FilledScanline, w.bounds).len();
    let r = time_mode(&w, TriangleMode::FilledScanline, 3, 1);
    assert_eq!(r.iterations, 3);
    assert_eq!(r.fragments_per_iteration, expected);
    assert!(r.elapsed_ns > 0);
    assert!(r.ns_per_iteration() > 0.0);
}

#[test]
fn run_benchmark_produces_rounds_per_mode_with_medians() {
    let w = build_workload();
    let cfg = BenchmarkConfig {
        warmup_rounds: 1,
        rounds: 3,
        iterations: 2,
    };
    let report = run_benchmark(&w, cfg).unwrap();
    assert_eq!(report.rounds.len(), 6);
    assert_eq!(report.rounds_for(TriangleMode::FilledBarycentric).len(), 3);
    assert_eq!(report.rounds_for(TriangleMode::FilledScanline).len(), 3);
    assert!(
        report
            .rounds
            .iter()
            .all(|r| r.fragments_per_iteration == report.fragments_per_iteration)
    );
    assert!(report.median_round_ns(TriangleMode::FilledScanline) > 0.0);
    assert_eq!(report.triangles, 207);
}

#[test]
fn median_handles_odd_and_even_counts() {
    assert_eq!(median_ns(&[5, 1, 3]), 3.0);
    assert_eq!(median_ns(&[4, 1, 3, 2]), 2.5);
    assert_eq!(median_ns(&[7]), 7.0);
    assert!(median_ns(&[]).is_nan());
}

#[test]
fn benchmark_flag_is_parsed() {
    let a = parse_args(["--benchmark-triangles".to_string()]).unwrap();
    assert!(a.benchmark);
    assert!(!parse_args(Vec::<String>::new()).unwrap().benchmark);
}

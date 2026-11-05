//! Laboratory 6 shader benchmark (separate from laboratory 5).

use rust_software_renderer::benchmark::{
    ShaderBenchConfig, run_shader_benchmark, write_shader_csv,
};
use rust_software_renderer::rasterization::TriangleMode;

fn tiny() -> ShaderBenchConfig {
    ShaderBenchConfig {
        warmup: 0,
        rounds: 1,
        width: 64,
        height: 48,
        threads: vec![1],
        time: 6.0,
    }
}

#[test]
fn shader_benchmark_writes_one_row_per_body_mode_and_thread_count() {
    let rows = run_shader_benchmark(&tiny()).unwrap();
    assert_eq!(rows.len(), 3 * 2);
    let dir = std::env::temp_dir().join(format!("lab6-shader-bench-{}", std::process::id()));
    let path = dir.join("shader_benchmark.csv");
    write_shader_csv(&rows, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        "body,mode,threads,width,height,rounds,fragments_shaded,median_raster_ms,median_shade_ms,median_total_ms"
    );
    assert_eq!(lines.len(), 1 + 6);
    for line in &lines[1..] {
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(cols.len(), 10, "{line}");
        assert_eq!(cols[2], "1");
        assert_eq!((cols[3], cols[4]), ("64", "48"));
        for c in &cols[7..] {
            assert!(c.parse::<f64>().unwrap() >= 0.0);
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filled_modes_shade_the_same_fragment_count() {
    let rows = run_shader_benchmark(&tiny()).unwrap();
    for pair in rows.chunks(2) {
        assert_eq!(pair[0].mode, TriangleMode::FilledBarycentric);
        assert_eq!(pair[1].mode, TriangleMode::FilledScanline);
        assert_eq!(pair[0].body, pair[1].body);
        assert_eq!(pair[0].fragments_shaded, pair[1].fragments_shaded);
        assert!(pair[0].fragments_shaded > 0);
    }
}

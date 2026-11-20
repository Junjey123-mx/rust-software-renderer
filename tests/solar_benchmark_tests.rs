//! Solar benchmark: scenarios A-J at a tiny size, no files written.

use rust_software_renderer::app::parse_args;
use rust_software_renderer::benchmark::solar::{SCENARIOS, scenario_scene};
use rust_software_renderer::benchmark::{SolarBenchConfig, run_solar_benchmark, write_solar_csv};
use rust_software_renderer::celestial::cli::{extract, validate};

fn tiny() -> SolarBenchConfig {
    SolarBenchConfig {
        width: 160,
        height: 120,
        warmup: 0,
        rounds: 1,
        threads: 2,
        time: 6.0,
    }
}

#[test]
fn every_scenario_renders_with_finite_times_and_the_expected_content() {
    let rows = run_solar_benchmark(&tiny()).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, SCENARIOS.iter().map(|(id, _)| *id).collect::<Vec<_>>());
    for r in &rows {
        assert!(
            r.median_ms.is_finite() && r.p95_ms >= r.median_ms && r.min_ms <= r.median_ms,
            "{r:?}"
        );
        assert_eq!((r.width, r.height, r.threads, r.rounds), (160, 120, 2, 1));
    }
    let get = |id: &str| rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(get("A").bodies, 0);
    assert_eq!(get("B").bodies, 1);
    assert_eq!(get("D").bodies, 3);
    // The star field never shades fragments: H and I shade the same.
    assert_eq!(get("H").fragments_shaded, get("I").fragments_shaded);
    // Facing empty space only the spacecraft is shaded, exactly as alone.
    assert_eq!(get("G").fragments_shaded, get("A").fragments_shaded);
    assert!(get("D").fragments_shaded > get("G").fragments_shaded);
    // Close approaches cover more of the screen than the distant view.
    assert!(get("E").fragments_shaded > get("C").fragments_shaded * 3);
    assert!(get("J").fragments_shaded > get("C").fragments_shaded * 3);
    assert!(get("F").fragments_shaded > get("C").fragments_shaded * 3);
}

#[test]
fn scenarios_are_reproducible_and_unknown_ids_are_rejected() {
    let a = scenario_scene("E", &tiny()).unwrap();
    let b = scenario_scene("E", &tiny()).unwrap();
    assert_eq!(a.ship, b.ship);
    assert_eq!(a.time(), 6.0);
    assert!(scenario_scene("Z", &tiny()).is_err());
    assert!(
        run_solar_benchmark(&SolarBenchConfig {
            rounds: 0,
            ..tiny()
        })
        .is_err()
    );
}

#[test]
fn the_csv_has_one_row_per_scenario() {
    let rows = run_solar_benchmark(&tiny()).unwrap();
    let dir = std::env::temp_dir().join(format!("solar_bench_{}", std::process::id()));
    let path = dir.join("solar_benchmark.csv");
    write_solar_csv(&rows, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 11);
    for line in &lines {
        // 11 original columns, then max_ms and peak_rss_kb appended at the end.
        assert_eq!(line.split(',').count(), 13, "one field per column: {line}");
    }
    assert!(lines[0].starts_with("scenario,name,width"));
    assert!(lines[1].starts_with("A,spacecraft alone,160,120,2,1,0,"));
    assert!(!text.contains("NaN") && !text.contains("inf"));
}

#[test]
fn the_cli_flag_is_exclusive_with_other_benchmarks() {
    let parse = |line: &str| {
        let (lab6, rest) = extract(line.split_whitespace().map(str::to_string).collect())?;
        let cli = parse_args(rest)?;
        validate(&cli, &lab6)?;
        Ok::<_, String>(lab6)
    };
    assert!(parse("--benchmark-solar").unwrap().benchmark_solar);
    assert!(parse("--benchmark-solar --benchmark-output /tmp/x").is_ok());
    assert_eq!(
        parse("--benchmark-solar --threads 1").unwrap().threads,
        Some(1)
    );
    assert!(parse("--benchmark-solar --benchmark-shaders").is_err());
    assert!(parse("--benchmark-solar --benchmark-triangles").is_err());
}

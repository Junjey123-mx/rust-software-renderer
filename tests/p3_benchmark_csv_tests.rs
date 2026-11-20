//! Project 3 solar benchmark CSV, version 2: the frame counters, level of
//! detail, seed and commit appended after the version 1 columns, read back
//! with a strict parser.

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{
    SOLAR_CSV_HEADER, SOLAR_CSV_VERSION, SolarBenchRow, run_solar_benchmark, write_solar_csv,
};
use std::collections::HashMap;

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

/// Rows of a version 2 CSV as `column -> value`, rejecting a wrong header
/// or a row with a different number of fields.
fn parse(text: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty CSV")?.split(',').collect();
    if header.join(",") != SOLAR_CSV_HEADER {
        return Err(format!("unexpected header {header:?}"));
    }
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() != header.len() {
                return Err(format!(
                    "{} fields, expected {}",
                    fields.len(),
                    header.len()
                ));
            }
            Ok(header
                .iter()
                .zip(fields)
                .map(|(h, f)| (h.to_string(), f.to_string()))
                .collect())
        })
        .collect()
}

fn csv(rows: &[SolarBenchRow]) -> String {
    let dir = std::env::temp_dir().join(format!("p3_csv_{}_{}", std::process::id(), rows.len()));
    let path = dir.join("solar_benchmark.csv");
    write_solar_csv(rows, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    text
}

#[test]
fn header_is_versioned() {
    let text = csv(&run_solar_benchmark(&tiny()).unwrap());
    assert!(text.starts_with(
        "scenario,name,width,height,threads,rounds,bodies,fragments_shaded,median_ms,p95_ms,min_ms,max_ms,peak_rss_kb,csv_version,"
    ));
    let rows = parse(&text).unwrap();
    assert_eq!(rows.len(), 10);
    for row in &rows {
        assert_eq!(row["csv_version"], SOLAR_CSV_VERSION.to_string());
        assert_eq!(row["seed"], "42");
    }
    let d = rows.iter().find(|r| r["scenario"] == "D").unwrap();
    assert_eq!(d["lod_levels"], "crown:-;entrance:-;house:-");
}

#[test]
fn a_row_with_a_missing_column_is_rejected() {
    let text = csv(&run_solar_benchmark(&tiny()).unwrap());
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let cut = lines[1].rfind(',').unwrap();
    lines[1].truncate(cut);
    assert!(parse(&lines.join("\n")).is_err());
    assert!(parse(&text.replacen("csv_version", "version", 1)).is_err());
}

#[test]
fn same_scenario_same_counters() {
    let a = run_solar_benchmark(&tiny()).unwrap();
    let b = run_solar_benchmark(&tiny()).unwrap();
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.stats, y.stats, "{}", x.id);
        assert_eq!(x.fragments_shaded, y.fragments_shaded, "{}", x.id);
    }
}

#[test]
fn g_has_only_ship_triangles() {
    // G faces empty space: every body culled, only the spacecraft (and its
    // thruster flames) drawn, exactly as in A (the spacecraft alone).
    let rows = run_solar_benchmark(&tiny()).unwrap();
    let (a, g) = (&rows[0], &rows[6]);
    assert_eq!((a.id, g.id), ("A", "G"));
    assert_eq!(g.stats.body_triangles(), 0);
    assert_eq!(g.stats.visible_bodies(), 0);
    assert_eq!(g.stats.objects_culled, g.stats.objects_tested - 1);
    assert_eq!(g.stats.triangles_submitted, a.stats.triangles_submitted);
    assert!(
        g.stats.triangles_submitted >= 708,
        "the ship has 708 triangles"
    );
}

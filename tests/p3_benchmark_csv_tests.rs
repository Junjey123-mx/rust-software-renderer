//! Project 3 solar benchmark CSV, version 2: the frame counters, level of
//! detail, seed and commit appended after the version 1 columns, read back
//! with a strict parser. Since C75 also the final benchmark
//! (`docs/results/project3/solar_benchmark_final.csv`): every case of both
//! systems, with and without culling, on 1 thread and on every core.
//!
//! To regenerate the final CSV (about 10 minutes):
//! `P3_FINAL_CSV=<file> cargo test --release --test p3_benchmark_csv_tests -- --ignored`.

use rust_software_renderer::benchmark::SolarBenchConfig;
use rust_software_renderer::benchmark::solar::{
    FINAL_CSV_HEADER, SOLAR_CSV_HEADER, SOLAR_CSV_VERSION, SolarBenchRow, append_final_csv,
    final_cases, run_final_benchmark, run_solar_benchmark, write_solar_csv,
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

const FINAL_CSV: &str = "docs/results/project3/solar_benchmark_final.csv";

/// Cases of `final_cases` missing from the final CSV `text`, for each of
/// its two thread counts (1 and every core), or why the CSV is malformed.
fn missing_cases(text: &str) -> Result<Vec<String>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(FINAL_CSV_HEADER) {
        return Err("unexpected header".into());
    }
    let columns = FINAL_CSV_HEADER.split(',').count();
    let mut seen = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != columns {
            return Err(format!("{} fields: {line}", f.len()));
        }
        let ms = |k: usize| f[k].parse::<f64>().map_err(|e| format!("{e}: {line}"));
        let (median, lo, hi) = (ms(9)?, ms(10)?, ms(11)?);
        if !(lo <= median && median <= hi && ms(12)? <= lo && hi <= ms(13)?) {
            return Err(format!("spread out of order: {line}"));
        }
        if f[5] != "3" || f[6] != "7" {
            return Err(format!("not 3 runs of 7 rounds: {line}"));
        }
        seen.push((
            f[0].to_string(),
            f[1].to_string(),
            f[2].to_string(),
            f[4].to_string(),
        ));
    }
    let mut threads: Vec<String> = seen.iter().map(|s| s.3.clone()).collect();
    threads.sort();
    threads.dedup();
    if threads.len() != 2 || !threads.contains(&"1".to_string()) {
        return Err(format!(
            "thread counts {threads:?}, expected 1 and every core"
        ));
    }
    let mut missing = Vec::new();
    for t in &threads {
        for (system, variant, id, _) in final_cases() {
            let key = (
                system.to_string(),
                variant.to_string(),
                id.to_string(),
                t.clone(),
            );
            if !seen.contains(&key) {
                missing.push(format!("{system}/{variant}/{id}/{t}"));
            }
        }
    }
    Ok(missing)
}

#[test]
fn final_csv_has_every_case() {
    let text = std::fs::read_to_string(FINAL_CSV).unwrap();
    assert_eq!(missing_cases(&text), Ok(vec![]));
    // Negative case: a scenario that is missing fails.
    let lines: Vec<&str> = text.lines().collect();
    let k = lines
        .iter()
        .position(|l| l.starts_with("p3,auto,M,"))
        .unwrap();
    let cut: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != k)
        .map(|(_, l)| *l)
        .collect();
    let missing = missing_cases(&cut.join("\n")).unwrap();
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing[0].starts_with("p3/auto/M/"));
    assert!(missing_cases(&text.replacen("system", "sys", 1)).is_err());
}

#[test]
fn final_benchmark_counts_every_case() {
    let config = SolarBenchConfig {
        width: 80,
        height: 60,
        ..tiny()
    };
    let rows = run_final_benchmark(&config, 2).unwrap();
    assert_eq!(rows.len(), final_cases().len());
    assert_eq!(rows.len(), 10 + 2 * 21);
    for row in &rows {
        assert_eq!(row.runs.len(), 2);
        // Without culling nothing is left out before rasterizing.
        if row.variant == "no-culling" {
            assert_eq!(row.runs[0].stats.objects_culled, 0, "{}", row.runs[0].id);
        }
    }
    // G faces empty space: culling leaves out every body.
    let g = |variant: &str| {
        rows.iter()
            .find(|r| r.system == "p3" && r.variant == variant && r.runs[0].id == "G")
            .unwrap()
            .runs[0]
            .stats
            .triangles_submitted
    };
    assert!(g("auto") < g("no-culling"));
    assert!(run_final_benchmark(&config, 0).is_err());
}

#[test]
#[ignore = "about 10 minutes: regenerates the final CSV at $P3_FINAL_CSV"]
fn regenerate_final_csv() {
    let path = std::env::var("P3_FINAL_CSV").expect("P3_FINAL_CSV=<file>");
    let path = std::path::Path::new(&path);
    let _ = std::fs::remove_file(path);
    for threads in [1, 0] {
        let config = SolarBenchConfig {
            threads,
            ..SolarBenchConfig::default()
        };
        append_final_csv(&run_final_benchmark(&config, 3).unwrap(), path).unwrap();
    }
}

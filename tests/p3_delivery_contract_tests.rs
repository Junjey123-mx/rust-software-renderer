//! Project 3 delivery contract: the five mandatory requirements R1-R5 of the
//! official PDF, read from `docs/project3_delivery_contract.md`.
//!
//! Every row marked `CUMPLE` must point at evidence that exists in the
//! repository; `PENDIENTE` rows declare what is still missing. A green suite
//! does not replace the teacher's review.

use rust_software_renderer::config::{AppConfig, SPHERE_MODEL_PATH};
use rust_software_renderer::io::load_obj;
use std::path::Path;

const CONTRACT: &str = "docs/project3_delivery_contract.md";

#[derive(Debug, PartialEq)]
struct Row {
    id: String,
    state: String,
    evidence: Vec<String>,
}

/// Rows of the R1-R5 table: `| Req | Requisito | Estado | Evidencia | Demostración |`.
fn contract_rows(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    let is_row = |l: &&str| {
        l.strip_prefix("| R")
            .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    };
    for line in text.lines().filter(is_row) {
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
        if cells.len() != 5 {
            return Err(format!("expected 5 cells, got {}: {line}", cells.len()));
        }
        let state = cells[2].trim().to_string();
        if !state.starts_with("CUMPLE") && state != "PENDIENTE" {
            return Err(format!("unknown state '{state}'"));
        }
        let evidence = cells[3]
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect();
        rows.push(Row {
            id: cells[0].trim().to_string(),
            state,
            evidence,
        });
    }
    Ok(rows)
}

fn rows() -> Vec<Row> {
    contract_rows(&std::fs::read_to_string(CONTRACT).unwrap()).unwrap()
}

#[test]
fn five_rows_present() {
    let ids: Vec<String> = rows().into_iter().map(|r| r.id).collect();
    assert_eq!(ids, ["R1", "R2", "R3", "R4", "R5"]);
}

#[test]
fn cumple_rows_have_existing_evidence() {
    for row in rows().iter().filter(|r| r.state.starts_with("CUMPLE")) {
        assert!(!row.evidence.is_empty(), "{row:?}");
        for path in &row.evidence {
            assert!(Path::new(path).exists(), "{}: missing {path}", row.id);
        }
    }
}

/// The R5 video of a row: the final tour, not the provisional clip.
fn r5_video(row: &Row) -> Option<&str> {
    row.evidence
        .iter()
        .map(String::as_str)
        .find(|p| p.ends_with(".mp4"))
}

#[test]
fn r5_video_is_final_tour() {
    let rows = rows();
    let r5 = rows.iter().find(|r| r.id == "R5").unwrap();
    assert_eq!(r5_video(r5), Some("docs/videos/project3/01-final-tour.mp4"));
    // Negative case: a row with only the provisional clip.
    let old = contract_rows(
        "| R5 | video | CUMPLE | `docs/videos/project3/00-provisional-tour.mp4` | x |",
    )
    .unwrap();
    assert_ne!(r5_video(&old[0]), r5_video(r5));
}

#[test]
fn malformed_tables_are_rejected() {
    assert!(contract_rows("| R1 | only | three |").is_err());
    assert!(contract_rows("| R1 | a | MAYBE | `x` | y |").is_err());
}

#[test]
fn models_parse_and_are_not_empty() {
    for path in [
        Path::new(SPHERE_MODEL_PATH).to_path_buf(),
        AppConfig::default().model_path,
    ] {
        let mesh = load_obj(&path).unwrap();
        assert!(mesh.triangle_count() > 0, "{}", path.display());
    }
}

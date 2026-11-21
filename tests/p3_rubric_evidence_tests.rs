//! Project 3 rubric claims (C73): the 17 rows of the official points menu,
//! read from `docs/project3_rubric_claims.md`. A `VERIFICADO` row points at
//! suites and captures that exist; nothing claims safe points without them.
//! A green suite is not the teacher's acceptance.

use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::system::spec::{FIFTH_PLANET, FifthPlanetStatus};
use std::path::Path;

const CLAIMS: &str = "docs/project3_rubric_claims.md";
const STATES: [&str; 3] = ["VERIFICADO", "PARCIAL", "NO IMPLEMENTADO"];
const CAPTURES: [&str; 3] = [".png", ".gif", ".mp4"];

#[derive(Debug, Clone)]
struct Row {
    id: u32,
    max: u32,
    state: String,
    suites: Vec<String>,
    command: String,
    captures: Vec<String>,
    safe: u32,
    conditional: u32,
}

/// Paths and names between backticks.
fn quoted(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// `| # | Rubro | Máx. | Estado | Cnn | Suites | Comando | Captura | Seguro | Condicionado |`.
fn parse(text: &str) -> Result<Vec<Row>, String> {
    let is_row = |l: &&str| {
        l.strip_prefix("| ")
            .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    };
    text.lines()
        .filter(is_row)
        .map(|line| {
            let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
            if cells.len() != 10 {
                return Err(format!("expected 10 cells, got {}: {line}", cells.len()));
            }
            let num = |k: usize| {
                cells[k]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| format!("{e}: {line}"))
            };
            let state = cells[3].trim().to_string();
            if !STATES.contains(&state.as_str()) {
                return Err(format!("unknown state '{state}'"));
            }
            Ok(Row {
                id: num(0)?,
                max: num(2)?,
                state,
                suites: quoted(cells[5]),
                command: quoted(cells[6]).concat(),
                captures: quoted(cells[7]),
                safe: num(8)?,
                conditional: num(9)?,
            })
        })
        .collect()
}

fn rows() -> Vec<Row> {
    parse(&std::fs::read_to_string(CLAIMS).unwrap()).unwrap()
}

/// Why `row` cannot stand, if it cannot.
fn check(row: &Row) -> Result<(), String> {
    if !(row.safe <= row.conditional && row.conditional <= row.max) {
        return Err(format!("row {}: points out of order", row.id));
    }
    if row.suites.is_empty() != (row.state == "NO IMPLEMENTADO") {
        return Err(format!(
            "row {}: suites without code or code without suites",
            row.id
        ));
    }
    if row.state == "NO IMPLEMENTADO" && row.conditional > 0 {
        return Err(format!("row {}: points for nothing", row.id));
    }
    if row.safe > 0 && row.state != "VERIFICADO" {
        return Err(format!("row {}: safe points without proof", row.id));
    }
    if row.state != "VERIFICADO" {
        return Ok(());
    }
    for suite in &row.suites {
        if !Path::new(&format!("tests/{suite}.rs")).exists() {
            return Err(format!("row {}: missing suite {suite}", row.id));
        }
        if !row.command.contains(&format!("--test {suite}")) {
            return Err(format!("row {}: the command skips {suite}", row.id));
        }
    }
    if !row.command.starts_with("cargo test") {
        return Err(format!("row {}: no command", row.id));
    }
    if row.captures.is_empty() {
        return Err(format!("row {}: no capture", row.id));
    }
    for capture in &row.captures {
        if !CAPTURES.iter().any(|e| capture.ends_with(e)) || !Path::new(capture).exists() {
            return Err(format!("row {}: missing capture {capture}", row.id));
        }
    }
    Ok(())
}

#[test]
fn seventeen_rows_sum_200() {
    let rows = rows();
    let ids: Vec<u32> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, (1..=17).collect::<Vec<_>>());
    assert_eq!(rows.iter().map(|r| r.max).sum::<u32>(), 200);
    // With the fifth planet pending the ceiling was 190; approved, at most
    // 200 (the menu counts five planets at most).
    let ceiling: u32 = rows.iter().map(|r| r.conditional).sum();
    match FIFTH_PLANET {
        FifthPlanetStatus::ArtPendingApproval => assert_eq!(ceiling, 190),
        FifthPlanetStatus::Approved(_) => assert!(ceiling <= 200),
    }
}

#[test]
fn verified_rows_have_evidence() {
    for row in rows() {
        check(&row).unwrap();
    }
    // Negative case: a verified row whose capture is not there fails.
    let mut row = rows()
        .into_iter()
        .find(|r| r.state == "VERIFICADO")
        .unwrap();
    row.captures = vec!["docs/screenshots/project3/does-not-exist.png".into()];
    assert!(check(&row).is_err());
    row.captures.clear();
    assert!(check(&row).is_err());
}

#[test]
fn no_claim_without_proof() {
    let rows = rows();
    for row in rows.iter().filter(|r| r.state != "VERIFICADO") {
        assert_eq!(row.safe, 0, "row {}", row.id);
    }
    // A claim on an unproven row, or a suite that does not exist, is refused.
    let mut row = rows.iter().find(|r| r.state == "PARCIAL").unwrap().clone();
    row.safe = 1;
    assert!(check(&row).is_err());
    let mut row = rows
        .iter()
        .find(|r| r.state == "VERIFICADO")
        .unwrap()
        .clone();
    row.suites.push("p3_no_such_suite".into());
    assert!(check(&row).is_err());
    let none = Row {
        suites: Vec::new(),
        state: "NO IMPLEMENTADO".into(),
        safe: 0,
        conditional: 0,
        ..row
    };
    assert!(check(&none).is_ok());
    assert!(
        check(&Row {
            conditional: 1,
            ..none
        })
        .is_err()
    );
    // Malformed tables are refused.
    assert!(parse("| 1 | a | 5 | MAYBE | b | c | d | e | 0 | 0 |").is_err());
    assert!(parse("| 1 | only | three |").is_err());
}

#[test]
fn planets_counted_without_star() {
    let planets = SystemSpec::project3().planets.len() as u32;
    assert_eq!(planets, 5, "the star is not a planet");
    let row = &rows()[0];
    assert_eq!((row.max, row.state.as_str()), (50, "VERIFICADO"));
    // Ten per planet, one close capture each, the star left out.
    assert_eq!(row.safe, 10 * planets);
    assert_eq!(row.captures.len() as u32, planets);
    assert!(row.captures.iter().all(|c| !c.contains("crown")));
}

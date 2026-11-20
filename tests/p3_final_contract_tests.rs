//! Project 3 release contract (C80): every `tests/p3_*` suite is listed in
//! `docs/project3_final_validation.md`, the laboratory 4-6 and
//! pre-project oracles and the two original models are byte for byte those
//! of the Gate's base `ea3b4a8` (and `LEGACY_A_J` that of C01), and no audit
//! file lives in the repository (audits are kept outside it).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const VALIDATION: &str = "docs/project3_final_validation.md";

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Suite names (`p3_..._tests`) in backticks in `text`.
fn listed_suites(text: &str) -> BTreeSet<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|s| s.starts_with("p3_") && s.ends_with("_tests"))
        .map(str::to_string)
        .collect()
}

fn suite_files() -> BTreeSet<String> {
    std::fs::read_dir("tests")
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            let stem = name.strip_suffix(".rs")?;
            stem.starts_with("p3_").then(|| stem.to_string())
        })
        .collect()
}

#[test]
fn all_p3_suites_registered() {
    let listed = listed_suites(&std::fs::read_to_string(VALIDATION).unwrap());
    let files = suite_files();
    assert!(files.len() >= 70, "{}", files.len());
    let missing: Vec<_> = files.difference(&listed).collect();
    let unknown: Vec<_> = listed.difference(&files).collect();
    assert!(missing.is_empty(), "suites not listed: {missing:?}");
    assert!(unknown.is_empty(), "listed but absent: {unknown:?}");
    // Negative case: a suite missing from the list is found.
    let mut short = listed.clone();
    short.remove("p3_final_contract_tests");
    assert_eq!(
        files.difference(&short).collect::<Vec<_>>(),
        ["p3_final_contract_tests"]
    );
}

/// FNV-1a of each file at the Gate's base `ea3b4a8` (`git show`).
const BASELINE: [(&str, u64); 11] = [
    ("tests/obj_loader_tests.rs", 0xcdda255dd3cf8f20),
    ("tests/wireframe_tests.rs", 0x5ab2d92a1c7c03d8),
    ("tests/lab5_baseline_tests.rs", 0xd4f3c884d2e8a2e3),
    ("tests/lab5_final_contract_tests.rs", 0x75e9026b86a4698a),
    (
        "tests/interactive_visual_regression_tests.rs",
        0x7bf08fa9d6e50a57,
    ),
    ("tests/lab6_final_contract_tests.rs", 0x553e2e0468a7e9e8),
    ("tests/lab6_regression_guard_tests.rs", 0xaac869ab2504f8e5),
    ("tests/lab6_sphere_asset_tests.rs", 0x831f433e45a33107),
    ("tests/preproject3_oracle_tests.rs", 0xec8958477fa9f2cd),
    ("assets/models/sphere.obj", 0xf05526000e2b7420),
    ("assets/models/Laboratorio3.obj", 0x89f91b441c62212e),
];

/// FNV-1a of the `LEGACY_A_J` table as frozen in C01 (`0f26c06`).
const LEGACY_A_J_BLOCK: u64 = 0x1b3ae97240ac536f;

fn legacy_block(text: &str) -> &str {
    let start = text.find("const LEGACY_A_J").unwrap();
    let end = start + text[start..].find("];").unwrap() + 2;
    &text[start..end]
}

#[test]
fn legacy_oracles_untouched() {
    for (path, hash) in BASELINE {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(fnv1a(&bytes), hash, "{path} changed since ea3b4a8");
    }
    let oracle = std::fs::read_to_string("tests/p3_frame_oracle_tests.rs").unwrap();
    assert_eq!(fnv1a(legacy_block(&oracle).as_bytes()), LEGACY_A_J_BLOCK);
    // Negative case: one changed byte is caught.
    let mut sphere = std::fs::read("assets/models/sphere.obj").unwrap();
    sphere[0] ^= 1;
    assert_ne!(fnv1a(&sphere), BASELINE[9].1);
}

/// An audit or Gate report by its name: those live outside the repository.
fn audit_like(path: &Path) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy().to_lowercase();
        name.starts_with("auditoria") || name.starts_with("audit")
    })
}

/// Every file of the working tree but build output and git's own data.
fn tree(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if dir == Path::new(".") && (name == "target" || name == ".git") {
            continue;
        }
        if path.is_dir() {
            tree(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn no_audit_files_in_repo() {
    let mut files = Vec::new();
    tree(Path::new("."), &mut files);
    assert!(files.len() > 100, "{}", files.len());
    let audits: Vec<_> = files.iter().filter(|p| audit_like(p)).collect();
    assert!(
        audits.is_empty(),
        "audit files in the repository: {audits:?}"
    );
    // Negative cases: an audit file or folder anywhere is found.
    assert!(audit_like(Path::new(
        "./docs/auditoria_gate_p3_final_validation.md"
    )));
    assert!(audit_like(Path::new("./auditorias/proyecto3/report.md")));
    assert!(audit_like(Path::new("./docs/audits/c80.md")));
    assert!(!audit_like(Path::new(
        "./docs/project3_final_validation.md"
    )));
}

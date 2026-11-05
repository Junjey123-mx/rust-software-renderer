//! The laboratory 6 documentation claims exactly what was implemented and
//! every evidence file it cites exists.

use std::path::{Path, PathBuf};

/// Effects implemented, tested and documented in this delivery.
const CLAIMED_EFFECTS: &[&str] = &["E01", "E02", "E03", "E04", "E05", "E06", "E07", "E08"];
/// Whether the optional shader benchmark (Gate C23) was executed.
const HAS_SHADER_BENCHMARK: bool = true;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap()
}

/// README text from `## Laboratorio 6` up to the next level-two heading.
fn lab6_section() -> String {
    let readme = read("README.md");
    let start = readme
        .find("## Laboratorio 6")
        .expect("README has a laboratory 6 section");
    let rest = &readme[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    rest[..end].to_string()
}

fn guide() -> String {
    read("docs/laboratorio6.md")
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/')
}

/// Every `docs/...` path mentioned anywhere in `text`.
fn docs_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut search = 0;
    while let Some(i) = text[search..].find("docs/") {
        let start = search + i;
        let path: String = text[start..]
            .chars()
            .take_while(|&c| is_path_char(c))
            .collect();
        let path = path.trim_end_matches('.').to_string();
        search = start + path.len().max(5);
        paths.push(path);
    }
    paths
}

/// Targets of markdown links `](target)` that are local files.
fn local_links(text: &str) -> Vec<String> {
    text.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .filter(|t| !t.starts_with("http") && !t.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn assert_exists(base: &Path, target: &str) {
    assert!(
        base.join(target).exists(),
        "missing {}",
        base.join(target).display()
    );
}

#[test]
fn readme_lab6_links_exist() {
    let section = lab6_section();
    let guide = guide();
    let mut checked = 0;
    for text in [&section, &guide] {
        for path in docs_paths(text) {
            assert_exists(&root(), &path);
            checked += 1;
        }
    }
    for link in local_links(&section) {
        assert_exists(&root(), &link);
        checked += 1;
    }
    for link in local_links(&guide) {
        assert_exists(&root().join("docs"), &link);
        checked += 1;
    }
    assert!(checked > 30, "only {checked} references checked");
}

#[test]
fn readme_claims_exactly_the_implemented_effects() {
    let section = lab6_section();
    let start = section
        .find("### Efectos solicitados")
        .expect("effects table subsection");
    let table = &section[start..];
    let end = table[4..].find("\n### ").map_or(table.len(), |i| i + 4);
    let mut ids: Vec<&str> = table[..end]
        .lines()
        .filter_map(|l| l.strip_prefix("| E0"))
        .map(|rest| &rest[..1])
        .collect();
    ids.sort();
    let claimed: Vec<String> = ids.iter().map(|d| format!("E0{d}")).collect();
    assert_eq!(
        claimed, CLAIMED_EFFECTS,
        "effect rows must be exactly the implemented set"
    );
    if CLAIMED_EFFECTS.len() < 8 {
        assert!(section.contains("No implementados en esta entrega"));
    }
}

#[test]
fn readme_lists_the_three_bodies() {
    let section = lab6_section();
    for name in ["The King's Crown", "Crimson Entrance", "House of Cards"] {
        assert!(section.contains(name), "{name}");
    }
    for scene in ["--scene crown", "--scene entrance", "--scene house"] {
        assert!(section.contains(scene), "{scene}");
    }
}

#[test]
fn benchmark_is_documented_only_when_measured() {
    let docs = format!("{}\n{}", lab6_section(), guide());
    let csv = root().join("docs/results/lab6/shader_benchmark.csv");
    if HAS_SHADER_BENCHMARK {
        assert!(csv.exists());
        assert!(docs.contains("--benchmark-shaders") && docs.contains("shader_benchmark.csv"));
    } else {
        assert!(!docs.contains("--benchmark-shaders"));
        assert!(!docs.contains("shader_benchmark.csv"));
    }
}

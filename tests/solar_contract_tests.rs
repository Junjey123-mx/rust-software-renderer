//! Contracts of the Camino al Proyecto 3 sources: the solar scene stays a
//! CPU renderer on raylib's window only, with the course assets untouched.
//! The laboratory six contract suites are not modified; this one scans the
//! new modules with its own needle list (specific raylib calls, so names
//! such as `draw_stars` are not false positives).

use rust_software_renderer::app::Scene;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every source added for the solar scene, with its text.
fn solar_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    rust_files(&root().join("src/solar"), &mut files);
    for extra in [
        "src/pipeline/transform.rs",
        "src/pipeline/perspective.rs",
        "src/benchmark/solar.rs",
    ] {
        files.push(root().join(extra));
    }
    assert!(files.len() >= 15, "{files:?}");
    files
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(&f).unwrap();
            (f, text)
        })
        .collect()
}

/// Raylib drawing, 3D or loading calls and external images or materials.
const FORBIDDEN: [&str; 17] = [
    "begin_drawing",
    "begin_mode3D",
    "Camera3D",
    "load_model",
    "draw_model",
    "draw_sphere",
    "draw_cube",
    "draw_pixel",
    "draw_circle",
    "draw_texture",
    "draw_line(",
    "load_texture",
    "load_image",
    "Image::load",
    "include_bytes!",
    ".mtl",
    "usemtl",
];

/// Violations of `FORBIDDEN` in a source text.
fn violations(text: &str) -> Vec<&'static str> {
    FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| text.contains(needle))
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[test]
fn the_scanner_detects_a_simulated_violation_and_ignores_our_names() {
    assert_eq!(
        violations("d.draw_texture(&t, 0, 0, Color::WHITE);"),
        ["draw_texture"]
    );
    assert_eq!(
        violations("rl.load_model(&thread, \"ship.obj\")"),
        ["load_model"]
    );
    assert!(violations("draw_stars(&mut fb, &field, &camera); line(a, b, c)").is_empty());
}

#[test]
fn solar_sources_never_draw_load_or_texture_with_raylib() {
    for (file, text) in solar_sources() {
        assert!(
            violations(&text).is_empty(),
            "{} uses {:?}",
            file.display(),
            violations(&text)
        );
    }
}

#[test]
fn the_dependencies_are_still_raylib_only() {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let section: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|l| !l.starts_with('['))
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert_eq!(section, vec!["raylib = \"5.5\""]);
}

#[test]
fn the_course_sphere_and_the_spacecraft_are_byte_identical() {
    let models = root().join("assets/models");
    let sphere = std::fs::read(models.join("sphere.obj")).unwrap();
    let ship = std::fs::read(models.join("Laboratorio3.obj")).unwrap();
    assert_eq!(
        (sphere.len(), fnv1a64(&sphere)),
        (16423, 0xf05526000e2b7420)
    );
    assert_eq!((ship.len(), fnv1a64(&ship)), (42865, 0x89f91b441c62212e));
    let mut names: Vec<String> = std::fs::read_dir(&models)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["Laboratorio3.obj", "sphere.obj"]);
}

#[test]
fn every_scene_is_still_recognised_next_to_solar() {
    for name in ["ship", "triangles", "crown", "entrance", "house", "solar"] {
        assert!(name.parse::<Scene>().is_ok(), "{name}");
    }
    assert_eq!("solar".parse::<Scene>().unwrap(), Scene::Solar);
}

#[test]
fn the_solar_scene_shades_on_the_cpu_through_the_shared_pipeline() {
    // The scene renders through the shaded stage and the CPU framebuffer:
    // its sources call the pipeline and never raylib's renderer.
    let scene = std::fs::read_to_string(root().join("src/solar/scene.rs")).unwrap();
    assert!(scene.contains("scratch.rasterize(") && scene.contains("shade_batch("));
    let instance = std::fs::read_to_string(root().join("src/solar/instance.rs")).unwrap();
    assert!(instance.contains("render_shaded_with_scratch("));
    let viewer = std::fs::read_to_string(root().join("src/solar/viewer.rs")).unwrap();
    assert!(viewer.contains("Presenter::new") && viewer.contains("presenter.present("));
}

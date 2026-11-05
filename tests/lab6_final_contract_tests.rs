//! Laboratory 6 contracts: official restrictions, exclusive symbols and
//! zero regressions of the spacecraft (R13-R16).

use rust_software_renderer::celestial::{Body, CelestialScene};
use rust_software_renderer::config::AppConfig;
use rust_software_renderer::geometry::Vertex;
use rust_software_renderer::io::load_obj;
use rust_software_renderer::pipeline::{
    Projection, ShadedOptions, Uniforms, ViewPlane, ViewPreset, render_with_mode,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
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

/// Every laboratory 6 source file with its text.
fn new_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    rust_files(&root().join("src/celestial"), &mut files);
    rust_files(&root().join("src/shading"), &mut files);
    files.push(root().join("src/pipeline/shaded.rs"));
    files.push(root().join("src/pipeline/surface.rs"));
    assert!(files.len() >= 20, "{files:?}");
    files
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(&f).unwrap();
            (f, text)
        })
        .collect()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap()
}

#[test]
fn new_sources_never_draw_or_load_with_raylib() {
    for (file, text) in new_sources() {
        for needle in [
            "draw_",
            "load_model",
            "begin_mode3D",
            "Camera3D",
            "get_camera_matrix",
        ] {
            assert!(!text.contains(needle), "{} uses {needle}", file.display());
        }
    }
}

#[test]
fn no_image_textures_or_external_materials() {
    for (file, text) in new_sources() {
        for needle in [
            "load_image",
            "load_texture",
            "Image::load",
            "include_bytes!",
            ".png",
            ".jpg",
            ".mtl",
            "usemtl",
        ] {
            assert!(!text.contains(needle), "{} uses {needle}", file.display());
        }
    }
}

#[test]
fn symbols_are_exclusive_per_body() {
    let mut entrance = Vec::new();
    rust_files(&root().join("src/celestial/entrance"), &mut entrance);
    assert!(entrance.len() >= 4);
    for file in entrance {
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            !text.contains("diamond_l1"),
            "{} uses the diamond",
            file.display()
        );
    }
    let house = read("src/celestial/house.rs");
    assert!(house.contains("contour_param") && !house.contains("spade("));
    let crown = read("src/celestial/crown.rs");
    assert!(!crown.contains("spade(") && !crown.contains("diamond_l1"));
    assert!(read("src/celestial/entrance/events.rs").contains("spade("));
}

#[test]
fn cargo_dependencies_are_unchanged() {
    let manifest = read("Cargo.toml");
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
fn assets_contain_only_obj_models() {
    let mut names = Vec::new();
    let mut stack = vec![root().join("assets")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                names.push(path.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    assert_eq!(names, vec!["Laboratorio3.obj", "sphere.obj"]);
}

fn render(body: Body, mode: TriangleMode, time: f32) -> (Framebuffer, Vec<Vertex>) {
    let mut scene = CelestialScene::new(body, ViewPreset::Oblique, 160, 120).unwrap();
    scene.time = time;
    let mut fb = Framebuffer::new(160, 120);
    scene
        .render(&mut fb, mode, ShadedOptions::default())
        .unwrap();
    (fb, scene.vertices)
}

#[test]
fn every_body_renders_in_every_mode() {
    for body in Body::ALL {
        for mode in TriangleMode::ALL {
            let (fb, _) = render(body, mode, 6.0);
            assert!(fb.count_non_background() > 500, "{body} {mode}");
        }
    }
}

#[test]
fn filled_modes_are_identical_for_every_body() {
    for body in Body::ALL {
        let (a, _) = render(body, TriangleMode::FilledBarycentric, 6.0);
        let (b, _) = render(body, TriangleMode::FilledScanline, 6.0);
        assert_eq!(a.to_rgba_bytes(), b.to_rgba_bytes(), "{body}");
    }
}

#[test]
fn geometry_is_identical_across_time_and_bodies() {
    let positions = |v: &[Vertex]| v.iter().map(|x| x.position).collect::<Vec<_>>();
    let (_, reference) = render(Body::KingsCrown, TriangleMode::FilledScanline, 0.0);
    for body in Body::ALL {
        for time in [0.0, 7.3, 30.0] {
            let (_, v) = render(body, TriangleMode::FilledScanline, time);
            assert_eq!(positions(&v), positions(&reference), "{body} t {time}");
        }
    }
}

#[test]
fn celestial_rendering_leaves_the_spacecraft_untouched() {
    let ship_hash = || {
        let ship = load_obj(&AppConfig::default().model_path)
            .unwrap()
            .expand_vertices(Vertex::DEFAULT_COLOR);
        let mut fb = Framebuffer::new(800, 600);
        render_with_mode(
            &mut fb,
            &Uniforms::default(),
            &ship,
            Projection::Fit {
                plane: ViewPlane::XZ,
                margin_ratio: 0.1,
            },
            TriangleMode::Wireframe,
        )
        .unwrap();
        fb.to_rgba_bytes()
            .iter()
            .fold(0xcbf29ce484222325_u64, |h, &b| {
                (h ^ b as u64).wrapping_mul(0x100000001b3)
            })
    };
    let before = ship_hash();
    for body in Body::ALL {
        render(body, TriangleMode::FilledScanline, 4.0);
    }
    assert_eq!(ship_hash(), before);
    assert_eq!(
        before, 0xf96077b19acbcd4c,
        "laboratory four golden fingerprint"
    );
}

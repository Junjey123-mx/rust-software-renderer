//! Error handling: every bad input yields a typed error, never a panic, and
//! never a substitute model.

use rust_software_renderer::app::{CliArgs, parse_args};
use rust_software_renderer::geometry::{Mesh, MeshError};
use rust_software_renderer::io::{ObjError, load_obj, parse_obj};
use rust_software_renderer::pipeline::{Projection, RenderError, Uniforms, ViewPlane, render};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use std::path::{Path, PathBuf};

type Check = fn(&ObjError) -> bool;

fn args(list: &[&str]) -> Result<CliArgs, String> {
    parse_args(list.iter().map(|s| s.to_string()))
}

#[test]
fn missing_and_wrong_paths_fail_clearly() {
    for p in ["nope.obj", "/definitely/not/here.obj", "assets/models"] {
        let err = load_obj(Path::new(p)).unwrap_err();
        assert!(matches!(err, ObjError::Io { .. }), "{p}: {err}");
        assert!(err.to_string().contains(p));
    }
}

#[test]
fn malformed_faces_and_indices_report_line_numbers() {
    let base = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\n";
    let cases: [(&str, Check); 5] = [
        ("f 1 2 3 4\n", |e| {
            matches!(
                e,
                ObjError::NonTriangularFace {
                    line: 5,
                    corners: 4
                }
            )
        }),
        ("f 1 2\n", |e| {
            matches!(
                e,
                ObjError::NonTriangularFace {
                    line: 5,
                    corners: 2
                }
            )
        }),
        ("f 0 1 2\n", |e| {
            matches!(e, ObjError::InvalidIndex { line: 5, .. })
        }),
        ("f 1 2 9\n", |e| {
            matches!(
                e,
                ObjError::IndexOutOfRange {
                    line: 5,
                    index: 9,
                    vertex_count: 4
                }
            )
        }),
        ("f a b c\n", |e| {
            matches!(e, ObjError::InvalidIndex { line: 5, .. })
        }),
    ];
    for (tail, check) in cases {
        let err = parse_obj(&format!("{base}{tail}")).unwrap_err();
        assert!(check(&err), "{tail:?} -> {err}");
        assert!(err.to_string().contains("line 5"), "{err}");
    }
}

#[test]
fn invalid_coordinates_are_rejected() {
    for src in [
        "v 1 2 x\nf 1 1 1\n",
        "v 1\nf 1 1 1\n",
        "v nan 0 0\nf 1 1 1\n",
        "v inf 0 0\nf 1 1 1\n",
    ] {
        let err = parse_obj(src).unwrap_err();
        assert!(
            matches!(err, ObjError::InvalidVertex { line: 1, .. }),
            "{src:?} -> {err}"
        );
    }
}

#[test]
fn empty_or_useless_files_are_rejected() {
    for src in ["", "\n\n", "# c\n", "v 1 2 3\n", "vn 0 0 1\nvt 0 0\n"] {
        assert!(
            matches!(parse_obj(src).unwrap_err(), ObjError::NoGeometry),
            "{src:?}"
        );
    }
}

#[test]
fn mesh_invariants_cannot_be_bypassed() {
    let v = vec![raylib::math::Vector3::zero(); 2];
    assert_eq!(
        Mesh::new(v.clone(), vec![0, 1, 2]).unwrap_err(),
        MeshError::IndexOutOfRange {
            index: 2,
            vertex_count: 2
        }
    );
    assert_eq!(
        Mesh::new(v.clone(), vec![0, 1]).unwrap_err(),
        MeshError::IndexCountNotMultipleOfThree(2)
    );
    assert_eq!(Mesh::new(v, vec![]).unwrap_err(), MeshError::Empty);
}

#[test]
fn render_errors_leave_the_framebuffer_untouched() {
    let mut fb = Framebuffer::new(64, 64);
    let projection = Projection::Fit {
        plane: ViewPlane::XZ,
        margin_ratio: 0.1,
    };
    assert!(matches!(
        render(&mut fb, &Uniforms::default(), &[], projection).unwrap_err(),
        RenderError::Mapping(_)
    ));
    let two = vec![
        rust_software_renderer::geometry::Vertex::new(raylib::math::Vector3::zero()),
        rust_software_renderer::geometry::Vertex::new(raylib::math::Vector3::one()),
    ];
    assert!(matches!(
        render(&mut fb, &Uniforms::default(), &two, projection).unwrap_err(),
        RenderError::Primitive(_)
    ));
    assert_eq!(fb.count_non_background(), 0);
}

#[test]
fn cli_arguments_are_validated() {
    assert_eq!(args(&[]).unwrap(), CliArgs::default());
    assert_eq!(
        args(&["--obj", "x.obj"]).unwrap(),
        CliArgs {
            obj: Some(PathBuf::from("x.obj")),
            screenshot: None,
            mode: TriangleMode::Wireframe,
            scene: Default::default(),
            benchmark: false,
        }
    );
    assert_eq!(
        args(&["--screenshot", "s.png"])
            .unwrap()
            .screenshot
            .as_deref(),
        Some("s.png")
    );
    assert!(args(&["--obj"]).is_err());
    assert!(args(&["--screenshot"]).is_err());
    assert!(args(&["--bogus"]).unwrap_err().contains("--bogus"));
}

#[test]
fn mode_argument_selects_a_triangle_mode() {
    assert_eq!(
        args(&[]).unwrap().mode,
        TriangleMode::Wireframe,
        "default is lab 4"
    );
    assert_eq!(
        args(&["--mode", "wireframe"]).unwrap().mode,
        TriangleMode::Wireframe
    );
    assert_eq!(
        args(&["--mode", "barycentric"]).unwrap().mode,
        TriangleMode::FilledBarycentric
    );
    assert_eq!(
        args(&["--mode", "scanline"]).unwrap().mode,
        TriangleMode::FilledScanline
    );
    // Last occurrence wins; other flags still parse alongside.
    let a = args(&[
        "--mode",
        "scanline",
        "--obj",
        "m.obj",
        "--mode",
        "barycentric",
    ])
    .unwrap();
    assert_eq!(a.mode, TriangleMode::FilledBarycentric);
    assert_eq!(a.obj.as_deref(), Some(Path::new("m.obj")));
    assert!(args(&["--mode"]).is_err());
    assert!(args(&["--mode", "filled"]).unwrap_err().contains("filled"));
    assert!(
        args(&["--mode", "Scanline"]).is_err(),
        "names are lowercase"
    );
}

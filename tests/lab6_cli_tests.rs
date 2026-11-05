//! Laboratory 6 command line: pre-pass extraction, validation and the
//! unchanged laboratory 4/5 command lines.

use rust_software_renderer::app::{CliArgs, Scene, USAGE, parse_args};
use rust_software_renderer::celestial::cli::{CelestialArgs, extract, validate};
use rust_software_renderer::celestial::viewer::celestial_mode;
use rust_software_renderer::celestial::{Body, DebugView};
use rust_software_renderer::rasterization::TriangleMode;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

fn full(args: &[&str]) -> Result<(CliArgs, CelestialArgs), String> {
    let (lab6, rest) = extract(strings(args))?;
    let cli = parse_args(rest)?;
    validate(&cli, &lab6)?;
    Ok((cli, lab6))
}

#[test]
fn extract_reads_every_lab6_option() {
    let (lab6, rest) = extract(strings(&[
        "--time",
        "8",
        "--freeze-time",
        "--light-azimuth",
        "-150",
        "--light-elevation",
        "5",
        "--set",
        "bump=0",
        "--set",
        "cloud_coverage=0.7",
        "--debug-view",
        "clouds",
        "--threads",
        "4",
    ]))
    .unwrap();
    assert!(rest.is_empty());
    assert_eq!(lab6.time, Some(8.0));
    assert!(lab6.freeze_time);
    assert_eq!(lab6.light_azimuth, Some(-150.0));
    assert_eq!(lab6.light_elevation, Some(5.0));
    assert_eq!(
        lab6.sets,
        vec![
            ("bump".to_string(), 0.0),
            ("cloud_coverage".to_string(), 0.7)
        ]
    );
    assert_eq!(lab6.debug_view, Some(DebugView::Clouds));
    assert_eq!(lab6.threads, Some(4));
    assert!(!lab6.is_empty());
}

#[test]
fn extract_preserves_other_arguments_in_order() {
    let (_, rest) = extract(strings(&[
        "--scene",
        "house",
        "--time",
        "2",
        "--view",
        "top",
        "--screenshot",
        "a.png",
    ]))
    .unwrap();
    assert_eq!(
        rest,
        strings(&["--scene", "house", "--view", "top", "--screenshot", "a.png"])
    );
}

#[test]
fn mode_presence_is_detected_without_consuming_it() {
    let (lab6, rest) = extract(strings(&["--scene", "crown", "--mode", "wireframe"])).unwrap();
    assert!(lab6.mode_explicit);
    assert!(lab6.is_empty(), "--mode is not a laboratory 6 option");
    assert_eq!(rest, strings(&["--scene", "crown", "--mode", "wireframe"]));
    let (cli, lab6) = full(&["--scene", "crown", "--mode", "wireframe"]).unwrap();
    assert_eq!(celestial_mode(&cli, &lab6), TriangleMode::Wireframe);
    let (cli, lab6) = full(&["--scene", "crown"]).unwrap();
    assert_eq!(
        celestial_mode(&cli, &lab6),
        TriangleMode::FilledScanline,
        "celestial scenes default to scanline"
    );
    assert_eq!(
        cli.mode,
        TriangleMode::Wireframe,
        "CliArgs default unchanged"
    );
}

#[test]
fn invalid_lab6_values_are_rejected() {
    for bad in [
        &["--time", "-1"][..],
        &["--time", "nan"],
        &["--light-elevation", "95"],
        &["--set", "novalue"],
        &["--set", "=1"],
        &["--set", "x=abc"],
        &["--threads", "0"],
        &["--debug-view", "foo"],
        &["--time"],
        &["--set"],
    ] {
        assert!(extract(strings(bad)).is_err(), "{bad:?} must be rejected");
    }
}

#[test]
fn celestial_scenes_parse() {
    for (name, body) in [
        ("crown", Body::KingsCrown),
        ("entrance", Body::CrimsonEntrance),
        ("house", Body::HouseOfCards),
    ] {
        let (cli, _) = full(&["--scene", name]).unwrap();
        assert_eq!(cli.scene, Scene::Celestial(body));
    }
}

#[test]
fn unknown_scene_error_still_names_the_input() {
    let err = full(&["--scene", "nebula"]).unwrap_err();
    assert!(err.contains("nebula") && err.contains("entrance"));
}

#[test]
fn lab6_options_require_a_celestial_scene() {
    assert!(full(&["--time", "3"]).is_err());
    assert!(full(&["--scene", "triangles", "--set", "bump=0"]).is_err());
    assert!(full(&["--scene", "ship", "--debug-view", "height"]).is_err());
    assert!(full(&["--scene", "entrance", "--set", "bump=0"]).is_ok());
}

#[test]
fn obj_cannot_override_the_course_sphere() {
    let err = full(&["--scene", "house", "--obj", "x.obj"]).unwrap_err();
    assert!(err.contains("--obj"));
}

#[test]
fn laboratory_five_command_lines_are_unchanged() {
    // Every command of the laboratory 4/5 README ("Cómo ejecutar" and the
    // interactive view section), copied verbatim.
    let lines: [&[&str]; 13] = [
        &[],
        &["--mode", "barycentric"],
        &["--scene", "triangles", "--mode", "wireframe"],
        &["--scene", "triangles", "--mode", "barycentric"],
        &["--scene", "triangles", "--mode", "scanline"],
        &[
            "--scene",
            "triangles",
            "--mode",
            "scanline",
            "--screenshot",
            "captura.png",
        ],
        &["--obj", "ruta/otro_modelo.obj"],
        &["--scene", "ship", "--interactive", "--mode", "scanline"],
        &[
            "--view",
            "top",
            "--mode",
            "barycentric",
            "--screenshot",
            "top.png",
        ],
        &["--benchmark-triangles"],
        &["--scene", "ship", "--interactive", "--mode", "wireframe"],
        &["--scene", "ship", "--interactive", "--mode", "barycentric"],
        &[
            "--benchmark-triangles",
            "--profile",
            "--profile-mode",
            "scanline",
        ],
    ];
    for line in lines {
        let (lab6, rest) = extract(strings(line)).unwrap();
        assert!(lab6.is_empty(), "{line:?}");
        assert_eq!(rest, strings(line));
        let cli = parse_args(rest).unwrap();
        assert!(validate(&cli, &lab6).is_ok());
        assert_eq!(cli, parse_args(strings(line)).unwrap());
    }
}

#[test]
fn usage_lists_celestial_scenes_and_keeps_old_hints() {
    assert!(USAGE.contains("--view") && USAGE.contains("1/2/3"));
    assert!(USAGE.contains("crown|entrance|house"));
    for option in [
        "--time",
        "--freeze-time",
        "--set",
        "--debug-view",
        "--threads",
    ] {
        assert!(USAGE.contains(option), "{option}");
    }
}

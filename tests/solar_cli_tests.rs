//! Command line of the solar scene; the earlier scenes keep their contracts.

use rust_software_renderer::app::{CliArgs, Scene, USAGE, parse_args};
use rust_software_renderer::celestial::cli::{extract, validate};
use rust_software_renderer::solar::viewer::{build_scene, solar_mode};

fn strings(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

fn parse(
    line: &str,
) -> Result<
    (
        CliArgs,
        rust_software_renderer::celestial::cli::CelestialArgs,
    ),
    String,
> {
    let (lab6, rest) = extract(strings(line))?;
    let cli = parse_args(rest)?;
    validate(&cli, &lab6)?;
    Ok((cli, lab6))
}

#[test]
fn solar_scene_parses_and_old_scenes_are_unchanged() {
    assert_eq!("solar".parse::<Scene>().unwrap(), Scene::Solar);
    assert_eq!("ship".parse::<Scene>().unwrap(), Scene::Ship);
    assert_eq!("triangles".parse::<Scene>().unwrap(), Scene::Triangles);
    assert!("crown".parse::<Scene>().is_ok());
    let err = "moon".parse::<Scene>().unwrap_err();
    assert!(err.contains("moon") && err.contains("solar"), "{err}");
    assert!(USAGE.contains("--scene solar") && USAGE.contains("crown|entrance|house"));
}

#[test]
fn solar_accepts_threads_mode_view_and_screenshot() {
    let (cli, lab6) =
        parse("--scene solar --threads 4 --mode barycentric --view top --screenshot s.png")
            .unwrap();
    assert_eq!(cli.scene, Scene::Solar);
    assert_eq!(lab6.threads, Some(4));
    assert_eq!(solar_mode(&cli, &lab6).name(), "barycentric");
    let (cli, lab6) = parse("--scene solar").unwrap();
    assert_eq!(solar_mode(&cli, &lab6).name(), "scanline");
}

#[test]
fn solar_rejects_recording_obj_and_single_body_options_for_now() {
    assert!(
        parse("--scene solar --record 2")
            .unwrap_err()
            .contains("--record")
    );
    assert!(
        parse("--scene solar --obj a.obj")
            .unwrap_err()
            .contains("--obj")
    );
    assert!(parse("--scene solar --time 3").is_err());
    assert!(parse("--scene solar --set cloud_coverage=0.5").is_err());
    // The single-body scenes still accept their options.
    assert!(parse("--scene entrance --time 3 --threads 2").is_ok());
    assert!(parse("--scene ship --threads 2").is_err());
}

#[test]
fn default_solar_scene_has_the_three_bodies_and_a_world_view() {
    let (cli, lab6) = parse("--scene solar").unwrap();
    let scene = build_scene(&cli, &lab6).unwrap();
    let names: Vec<&str> = scene.bodies.iter().map(|b| b.body().name()).collect();
    assert_eq!(names, ["crown", "entrance", "house"]);
    assert_eq!(scene.view.camera.target, raylib::math::Vector3::zero());
    assert!(scene.view.scale() > 0.0);
}

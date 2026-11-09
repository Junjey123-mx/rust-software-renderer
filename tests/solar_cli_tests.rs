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
    assert!(parse("--scene solar --freeze-time").is_err());
    assert!(parse("--scene solar --debug-view clouds").is_err());
    let err = parse("--scene solar --set cloud_coverage=0.5").unwrap_err();
    assert!(err.contains("prefix"), "{err}");
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

#[test]
fn solar_time_and_prefixed_overrides_reach_the_right_instance() {
    let (cli, lab6) = parse(
        "--scene solar --time 3.5 --set entrance.cloud_coverage=0.7 --set house.vortex_count=1",
    )
    .unwrap();
    let scene = build_scene(&cli, &lab6).unwrap();
    assert!((scene.time() - 3.5).abs() < 1e-6);
    let entrance = &scene.bodies[1];
    assert_eq!(entrance.material.get("cloud_coverage"), Some(0.7));
    assert_eq!(scene.bodies[2].material.get("vortex_count"), Some(1.0));
    assert_eq!(
        scene.bodies[0].material.get("cloud_coverage"),
        None,
        "crown untouched"
    );
    // Positions follow the orbits at t = 3.5, not the t = 0 layout.
    assert!(entrance.transform.position.z.abs() > 0.1);

    let (cli, lab6) = parse("--scene solar --set moon.x=1").unwrap();
    assert!(
        build_scene(&cli, &lab6)
            .unwrap_err()
            .to_string()
            .contains("moon")
    );
    let (cli, lab6) = parse("--scene solar --set crown.cloud_coverage=1").unwrap();
    assert!(build_scene(&cli, &lab6).is_err(), "crown has no clouds");
    let (cli, lab6) = parse("--scene solar --set entrance.cloud_coverage=9").unwrap();
    assert!(build_scene(&cli, &lab6).is_err(), "out of range");
    // The single-body syntax is unchanged.
    assert!(parse("--scene entrance --set cloud_coverage=0.7").is_ok());
}

#[test]
fn default_layout_keeps_the_bodies_apart_at_time_zero() {
    let (cli, lab6) = parse("--scene solar").unwrap();
    let scene = build_scene(&cli, &lab6).unwrap();
    let r = rust_software_renderer::solar::SPHERE_RADIUS;
    let radii: Vec<f32> = scene.bodies.iter().map(|b| b.transform.scale * r).collect();
    for i in 0..3 {
        for j in i + 1..3 {
            let d =
                (scene.bodies[i].transform.position - scene.bodies[j].transform.position).length();
            assert!(d > radii[i] + radii[j] + 0.3, "{i} and {j} too close: {d}");
        }
    }
    assert_eq!(
        scene.bodies[0].transform.position,
        raylib::math::Vector3::zero()
    );
    assert!(scene.extent() <= rust_software_renderer::solar::viewer::WORLD_RADIUS);
}

#[test]
fn solar_camera_option_defaults_to_perspective_and_is_solar_only() {
    use rust_software_renderer::solar::CameraMode;
    let (cli, lab6) = parse("--scene solar").unwrap();
    assert_eq!(
        build_scene(&cli, &lab6).unwrap().camera_mode,
        CameraMode::Perspective
    );
    let (cli, lab6) = parse("--scene solar --camera orbit").unwrap();
    assert_eq!(
        build_scene(&cli, &lab6).unwrap().camera_mode,
        CameraMode::Orbit
    );
    assert!(
        parse("--scene solar --camera fisheye")
            .unwrap_err()
            .contains("fisheye")
    );
    assert!(
        parse("--scene entrance --camera orbit")
            .unwrap_err()
            .contains("--camera")
    );
    assert!(USAGE.contains("--camera"));
}

#[test]
fn star_options_are_solar_only_and_shape_the_scene_sky() {
    let (cli, lab6) = parse("--scene solar --stars 0").unwrap();
    assert!(build_scene(&cli, &lab6).unwrap().stars.stars.is_empty());
    let (cli, lab6) = parse("--scene solar --stars 500 --star-seed 7").unwrap();
    let a = build_scene(&cli, &lab6).unwrap().stars;
    assert_eq!((a.seed, a.stars.len()), (7, 500));
    let (cli, lab6) = parse("--scene solar --stars 3000 --star-density clustered").unwrap();
    assert!(build_scene(&cli, &lab6).unwrap().stars.stars.len() < 3000);
    let (cli, lab6) = parse("--scene solar").unwrap();
    assert_eq!(build_scene(&cli, &lab6).unwrap().stars.stars.len(), 2000);
    assert!(parse("--scene solar --stars -1").is_err());
    assert!(
        parse("--scene solar --stars 999999")
            .unwrap_err()
            .contains("at most")
    );
    assert!(parse("--scene solar --star-seed x").is_err());
    assert!(
        parse("--scene crown --stars 10")
            .unwrap_err()
            .contains("--scene solar")
    );
    assert!(USAGE.contains("--stars"));
}

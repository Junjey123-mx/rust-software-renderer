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
fn solar_rejects_obj_and_single_body_options_and_records_the_flight() {
    assert!(
        parse("--scene solar --record 2").is_ok(),
        "the solar recorder exists since C39"
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
fn the_layout_reserves_room_for_a_ring_around_the_gas_giant() {
    use rust_software_renderer::celestial::Body;
    use rust_software_renderer::solar::scene::SHIP_SPAWN;
    use rust_software_renderer::solar::viewer::WORLD_RADIUS;
    // Outer radius of a future ring, in planet radii (Saturn's main rings
    // end near 2.3).
    const RING_RESERVE: f32 = 2.2;
    let (cli, lab6) = parse("--scene solar").unwrap();
    let scene = build_scene(&cli, &lab6).unwrap();
    let find = |body| scene.bodies.iter().position(|b| b.body() == body).unwrap();
    let (house, entrance, crown) = (
        find(Body::HouseOfCards),
        find(Body::CrimsonEntrance),
        find(Body::KingsCrown),
    );
    let orbit = |i: usize| scene.bodies[i].orbit.unwrap().radius;
    let ring = RING_RESERVE * scene.radius_of(house);
    // Coplanar circular orbits: the nearest House and Crimson Entrance ever
    // get is the difference of their orbit radii.
    let ring_gap = orbit(house) - orbit(entrance) - ring - scene.radius_of(entrance);
    assert!(
        ring_gap > 0.9,
        "ring reserve {ring_gap} from Crimson Entrance"
    );
    let inner_gap = orbit(entrance) - scene.radius_of(entrance) - scene.radius_of(crown);
    assert!(
        inner_gap > 1.2,
        "Crimson Entrance {inner_gap} from the crown"
    );
    assert!(
        orbit(house) + ring <= WORLD_RADIUS,
        "the orbit view frames the ring"
    );
    assert!(
        SHIP_SPAWN.length() > orbit(house) + ring + 0.5,
        "the spacecraft starts outside the ring"
    );
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
    let (cli, lab6) =
        parse("--scene solar --stars 500 --star-seed 7 --star-density uniform").unwrap();
    let a = build_scene(&cli, &lab6).unwrap().stars;
    assert_eq!((a.seed, a.stars.len()), (7, 500));
    let (cli, lab6) = parse("--scene solar --stars 3000 --star-density clustered").unwrap();
    assert!(build_scene(&cli, &lab6).unwrap().stars.stars.len() < 3000);
    // By default 8000 clustered candidates keep a little over half.
    let (cli, lab6) = parse("--scene solar").unwrap();
    let kept = build_scene(&cli, &lab6).unwrap().stars.stars.len();
    assert!((3000..6000).contains(&kept), "{kept}");
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

#[test]
fn solar_quality_defaults_to_full_and_is_solar_only() {
    use rust_software_renderer::solar::Quality;
    let (cli, lab6) = parse("--scene solar").unwrap();
    assert_eq!(build_scene(&cli, &lab6).unwrap().quality, Quality::Full);
    let (cli, lab6) = parse("--scene solar --solar-quality adaptive").unwrap();
    assert_eq!(build_scene(&cli, &lab6).unwrap().quality, Quality::Adaptive);
    assert!(parse("--scene house --solar-quality adaptive").is_err());
    assert!(parse("--scene solar --solar-quality max").is_err());
}

#[test]
fn nebula_option_is_solar_only_and_sets_the_backdrop_intensity() {
    let (cli, lab6) = parse("--scene solar").unwrap();
    assert_eq!(build_scene(&cli, &lab6).unwrap().nebula.intensity, 1.0);
    let (cli, lab6) = parse("--scene solar --nebula 0.4 --star-seed 9").unwrap();
    let nebula = build_scene(&cli, &lab6).unwrap().nebula;
    assert_eq!(
        (nebula.intensity, nebula.seed),
        (0.4, 9),
        "one seed for the sky"
    );
    let (cli, lab6) = parse("--scene solar --nebula 0").unwrap();
    assert_eq!(build_scene(&cli, &lab6).unwrap().nebula.intensity, 0.0);
    for bad in ["2", "-0.1", "x", "NaN"] {
        assert!(
            parse(&format!("--scene solar --nebula {bad}")).is_err(),
            "{bad}"
        );
    }
    assert!(
        parse("--scene crown --nebula 0.5")
            .unwrap_err()
            .contains("--scene solar")
    );
    assert!(USAGE.contains("--nebula"));
}

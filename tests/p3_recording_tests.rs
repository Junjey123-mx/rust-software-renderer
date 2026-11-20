//! Project 3 delivery tour (`--timeline tour`): one continuous flight with
//! no quick travel and no camera switch — space flight, approach, a full
//! lap of local flight around a planet and a departure — simulated frame by
//! frame exactly as the recorder plays it, without rendering.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::flight::{BOOST, FlightMode, LOCAL_SPEED, RADIAL_SPEED};
use rust_software_renderer::solar::input::SolarAction;
use rust_software_renderer::solar::record::{
    SolarTimeline, TOUR_SECONDS, TOUR_SPACE_SECONDS, prepare_timeline, script_frame, tour_actions,
};
use rust_software_renderer::solar::{SolarMeshes, SolarScene};
use std::f32::consts::TAU;

fn scene(time: f32) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::default_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap();
    s.clock.time = time;
    s.update();
    s
}

/// One played frame: ship position, flight mode, rig, toggle messages.
struct Frame {
    ship: Vector3,
    flight: FlightMode,
    rig: CameraRig,
    messages: Vec<String>,
}

fn play(time: f32, fps: u32) -> (SolarScene, Vec<Frame>) {
    let mut s = prepare_timeline(&scene(time), SolarTimeline::Tour, fps);
    let start = s.time();
    let mut paused_at = None;
    let frames = (TOUR_SECONDS * fps as f32).ceil() as usize;
    let played = (0..frames)
        .map(|f| {
            let (_, messages) =
                script_frame(&mut s, SolarTimeline::Tour, f, fps, start, &mut paused_at);
            Frame {
                ship: s.ship.position,
                flight: s.flight,
                rig: s.rig,
                messages,
            }
        })
        .collect();
    (s, played)
}

fn planet_index(frames: &[Frame]) -> usize {
    frames
        .iter()
        .find_map(|f| match f.flight {
            FlightMode::Local { planet, .. } => Some(planet),
            FlightMode::Space => None,
        })
        .expect("the tour never entered local flight")
}

#[test]
fn tour_parses_and_lasts_20_to_60_seconds() {
    assert_eq!("tour".parse::<SolarTimeline>(), Ok(SolarTimeline::Tour));
    assert!((20.0..=60.0).contains(&TOUR_SECONDS), "{TOUR_SECONDS}");
}

#[test]
fn tour_has_no_quick_travel_pause_or_camera_switch() {
    let fps = 30;
    let frames = (TOUR_SECONDS * fps as f32).ceil() as usize;
    let mut toggles = 0;
    for f in 0..frames {
        let t = f as f32 / fps as f32;
        let previous = if f == 0 { -1.0 } else { t - 1.0 / fps as f32 };
        for action in tour_actions(previous, t) {
            assert!(
                !matches!(
                    action,
                    SolarAction::Warp(_) | SolarAction::ToggleCamera | SolarAction::TogglePause
                ),
                "{action:?} at {t} s"
            );
            toggles += (action == SolarAction::ToggleLocal) as usize;
        }
    }
    assert_eq!(toggles, 2, "enter and leave local flight once");
}

#[test]
fn tour_enters_local_flight_once_and_leaves_it() {
    for (time, fps) in [(0.0, 30), (37.0, 30), (0.0, 60)] {
        let (_, frames) = play(time, fps);
        let messages: Vec<&String> = frames.iter().flat_map(|f| &f.messages).collect();
        assert_eq!(messages.len(), 2, "t0 {time}, {fps} fps: {messages:?}");
        let entered = (TOUR_SPACE_SECONDS * fps as f32).round() as usize;
        assert!(matches!(frames[entered - 1].flight, FlightMode::Space));
        assert!(
            matches!(frames[entered].flight, FlightMode::Local { .. }),
            "t0 {time}, {fps} fps: {messages:?}"
        );
        assert!(matches!(frames.last().unwrap().flight, FlightMode::Space));
        assert!(frames.iter().all(|f| f.rig == CameraRig::Follow));
    }
}

#[test]
fn local_flight_completes_a_full_lap() {
    for (time, fps) in [(0.0, 30), (37.0, 30), (0.0, 60)] {
        let mut s = prepare_timeline(&scene(time), SolarTimeline::Tour, fps);
        let start = s.time();
        let mut paused_at = None;
        let frames = (TOUR_SECONDS * fps as f32).ceil() as usize;
        let mut previous: Option<Vector3> = None;
        let mut swept = 0.0;
        for f in 0..frames {
            script_frame(&mut s, SolarTimeline::Tour, f, fps, start, &mut paused_at);
            let FlightMode::Local { planet, .. } = s.flight else {
                continue;
            };
            let radial = (s.ship.position - s.bodies[planet].transform.position).normalized();
            if let Some(p) = previous {
                swept += p.dot(radial).clamp(-1.0, 1.0).acos();
            }
            previous = Some(radial);
        }
        assert!(swept >= TAU, "t0 {time}, {fps} fps: swept {swept} rad");
    }
}

#[test]
fn ship_path_is_continuous_and_never_enters_a_body() {
    for (time, fps) in [(0.0, 30), (37.0, 30), (0.0, 60)] {
        let (s, frames) = play(time, fps);
        let planet_speed = s
            .bodies
            .iter()
            .filter_map(|b| b.orbit)
            .map(|o| o.speed.abs() * o.radius)
            .fold(0.0, f32::max);
        let dt = 1.0 / fps as f32;
        let bound = (s.ship.move_speed * BOOST + planet_speed + LOCAL_SPEED + RADIAL_SPEED) * dt;
        let first = prepare_timeline(&scene(time), SolarTimeline::Tour, fps)
            .ship
            .position;
        let mut last = first;
        for (f, frame) in frames.iter().enumerate() {
            let step = (frame.ship - last).length();
            assert!(step <= bound * 1.05, "frame {f}: jumped {step} > {bound}");
            last = frame.ship;
        }
        // Replay to check the clearance against every body at every frame.
        let mut s = prepare_timeline(&scene(time), SolarTimeline::Tour, fps);
        let start = s.time();
        let mut paused_at = None;
        for f in 0..frames.len() {
            script_frame(&mut s, SolarTimeline::Tour, f, fps, start, &mut paused_at);
            for i in 0..s.bodies.len() {
                let gap =
                    (s.ship.position - s.bodies[i].transform.position).length() - s.radius_of(i);
                assert!(gap > 0.45, "t0 {time}, frame {f}: {gap} from body {i}");
            }
        }
    }
}

#[test]
fn departure_ends_far_from_the_planet() {
    let (s, frames) = play(0.0, 30);
    let planet = planet_index(&frames);
    let leave = frames
        .iter()
        .rposition(|f| matches!(f.flight, FlightMode::Local { .. }))
        .unwrap();
    let center = s.bodies[planet].transform.position;
    let end = (s.ship.position - center).length() - s.radius_of(planet);
    assert!(end > 10.0, "ended {end} above the planet");
    assert!(leave + 1 < frames.len());
}

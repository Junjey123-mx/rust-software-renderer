//! Project 3 final tour (`--timeline final-tour`, C77): the delivery tour
//! around Crimson Entrance, then a cruise past the clover planet and the
//! Hearts planet and a departure, steered only by flight keys and mouse
//! turns under the same physics, in one take of at most 60 s: no quick
//! travel, no camera switch, no jump. Simulated frame by frame exactly as
//! the recorder plays it, without rendering.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::camera::CameraRig;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::flight::{BOOST, FlightMode};
use rust_software_renderer::solar::input::SolarAction;
use rust_software_renderer::solar::record::{
    FINAL_TOUR_SECONDS, FINAL_VISITS, SolarTimeline, prepare_timeline, script_frame,
};
use rust_software_renderer::solar::system::PlanetId;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};
use std::f32::consts::TAU;

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

/// One played frame.
struct Frame {
    actions: Vec<SolarAction>,
    ship: Vector3,
    flight: FlightMode,
    rig: CameraRig,
    bodies: Vec<Vector3>,
    /// Smallest gap between the ship's sphere and a body's highest ground.
    gap: f32,
    /// Distance of the ship to the ground of each `FINAL_VISITS` planet.
    visits: Vec<f32>,
    flames: bool,
}

fn play(fps: u32) -> (SolarScene, Vec<Frame>) {
    let mut s = prepare_timeline(&scene(), SolarTimeline::FinalTour, fps);
    let start = s.time();
    let mut paused_at = None;
    let frames = (FINAL_TOUR_SECONDS * fps as f32).ceil() as usize;
    let visits: Vec<usize> = FINAL_VISITS
        .iter()
        .map(|(id, _)| s.body_index(*id).unwrap())
        .collect();
    let played = (0..frames)
        .map(|f| {
            let (actions, _) = script_frame(
                &mut s,
                SolarTimeline::FinalTour,
                f,
                fps,
                start,
                &mut paused_at,
            );
            let ground = |i: usize| {
                let relief = s.relief_of(i).map_or(0.0, |r| r.field.max_height());
                (s.ship.position - s.bodies[i].transform.position).length()
                    - s.radius_of(i) * (1.0 + relief)
            };
            Frame {
                actions,
                ship: s.ship.position,
                flight: s.flight,
                rig: s.rig,
                bodies: s.bodies.iter().map(|b| b.transform.position).collect(),
                gap: (0..s.bodies.len())
                    .map(|i| ground(i) - s.ship.radius())
                    .fold(f32::MAX, f32::min),
                visits: visits.iter().map(|&i| ground(i)).collect(),
                flames: f % 15 == 0
                    && s.effect_draws(160, 120, 1)
                        .iter()
                        .any(|d| matches!(d.shader, EffectShader::Flame(_))),
            }
        })
        .collect();
    (s, played)
}

/// Why a script is not one continuous take: quick travel or a camera
/// switch in any frame.
fn cuts(script: &[Vec<SolarAction>]) -> Vec<String> {
    script
        .iter()
        .enumerate()
        .flat_map(|(f, actions)| {
            actions
                .iter()
                .filter(|a| matches!(a, SolarAction::Warp(_) | SolarAction::ToggleCamera))
                .map(move |a| format!("frame {f}: {a:?}"))
        })
        .collect()
}

#[test]
fn no_warp_no_cut() {
    assert_eq!(
        "final-tour".parse::<SolarTimeline>(),
        Ok(SolarTimeline::FinalTour)
    );
    let (_, frames) = play(30);
    let script: Vec<Vec<SolarAction>> = frames.iter().map(|f| f.actions.clone()).collect();
    assert_eq!(cuts(&script), Vec::<String>::new());
    assert!(frames.iter().all(|f| f.rig == CameraRig::Follow));
    // Negative case: a quick travel anywhere in the script is a cut.
    let mut warped = script.clone();
    warped[900].push(SolarAction::Warp(2));
    assert_eq!(cuts(&warped).len(), 1);
}

#[test]
fn full_local_lap() {
    let (_, frames) = play(30);
    let mut swept = 0.0;
    let mut previous: Option<Vector3> = None;
    let mut planet = None;
    for f in &frames {
        let FlightMode::Local { planet: p, .. } = f.flight else {
            continue;
        };
        planet = Some(p);
        let radial = (f.ship - f.bodies[p]).normalized();
        if let Some(r) = previous {
            swept += r.dot(radial).clamp(-1.0, 1.0).acos();
        }
        previous = Some(radial);
    }
    assert_eq!(planet, Some(1), "around Crimson Entrance");
    assert!(swept >= TAU, "swept {swept} rad");
    // Local flight is entered once and left once.
    let local: Vec<bool> = frames
        .iter()
        .map(|f| matches!(f.flight, FlightMode::Local { .. }))
        .collect();
    assert_eq!(local.windows(2).filter(|w| w[0] != w[1]).count(), 2);
}

#[test]
fn continuous_positions() {
    for fps in [30, 60] {
        let (s, frames) = play(fps);
        let planet_speed = s
            .bodies
            .iter()
            .filter_map(|b| b.orbit)
            .map(|o| o.speed.abs() * o.radius)
            .fold(0.0, f32::max);
        let local = s.flight_scale(1);
        let dt = 1.0 / fps as f32;
        let bound =
            (s.ship.move_speed * BOOST + planet_speed + local.local_speed + local.radial_speed)
                * dt;
        let mut last = prepare_timeline(&scene(), SolarTimeline::FinalTour, fps)
            .ship
            .position;
        for (f, frame) in frames.iter().enumerate() {
            let step = (frame.ship - last).length();
            assert!(
                step <= bound * 1.05,
                "{fps} fps, frame {f}: {step} > {bound}"
            );
            last = frame.ship;
            // Never into a planet's highest ground (nor the star).
            assert!(frame.gap > -1e-3, "{fps} fps, frame {f}: {}", frame.gap);
        }
    }
}

#[test]
fn orbits_and_thrusters_visible() {
    let (s, frames) = play(30);
    assert!(s.gravity, "gravity on along the whole tour");
    // Every planet orbits during the take.
    let (first, last) = (&frames[0].bodies, &frames.last().unwrap().bodies);
    for i in 1..first.len() {
        assert!((last[i] - first[i]).length() > 1.0, "body {i}");
    }
    // Both visits come close: under two radii of their ground.
    for (k, (id, at)) in FINAL_VISITS.iter().enumerate() {
        let i = s.body_index(*id).unwrap();
        let closest = frames
            .iter()
            .skip((*at * 30.0) as usize)
            .map(|f| f.visits[k])
            .fold(f32::MAX, f32::min);
        assert!(closest < 2.0 * s.radius_of(i), "{id:?}: {closest}");
        assert!(closest > 0.0, "{id:?}: {closest}");
    }
    assert_eq!(FINAL_VISITS[0].0, PlanetId("clover"));
    assert_eq!(FINAL_VISITS[1].0, PlanetId("hearts"));
    // The thrusters burn in view, and the light-speed bursts happen.
    assert!(frames.iter().filter(|f| f.flames).count() > 10);
    let boosts = frames
        .windows(2)
        .filter(|w| {
            !w[0].actions.contains(&SolarAction::Boost)
                && w[1].actions.contains(&SolarAction::Boost)
        })
        .count();
    assert!(boosts >= 4, "{boosts} bursts");
}

#[test]
fn under_60s() {
    assert!((20.0..=60.0).contains(&FINAL_TOUR_SECONDS));
    let (s, frames) = play(30);
    assert_eq!(frames.len(), (FINAL_TOUR_SECONDS * 30.0).ceil() as usize);
    // It ends far from the last planet, leaving at light speed.
    let hearts = s.body_index(PlanetId("hearts")).unwrap();
    let end = (s.ship.position - s.bodies[hearts].transform.position).length();
    assert!(end > 10.0 * s.radius_of(hearts), "{end}");
    assert!(frames.last().unwrap().actions.contains(&SolarAction::Boost));
}

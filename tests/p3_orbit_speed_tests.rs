//! Project 3 orbital speeds: linear speeds bounded by a third of the
//! spacecraft's cruise, so every planet can be caught without quick travel,
//! and positions that depend on the simulated time only.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::flight::FlightInput;
use rust_software_renderer::solar::input::SolarAction;
use rust_software_renderer::solar::system::spec::MAX_ORBIT_LINEAR_SPEED;
use rust_software_renderer::solar::system::{ScaleProfile, SystemSpec};
use rust_software_renderer::solar::{Orbit, SolarMeshes, SolarScene};

fn project3() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(80, 60, 0.1),
    )
    .unwrap();
    SolarScene::project3_system(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
    )
    .unwrap()
}

#[test]
fn omega_equals_v_over_r() {
    let o = Orbit::from_linear(14.0, 1.0, 0.3).unwrap();
    assert_eq!(o, Orbit::new(14.0, 1.0 / 14.0, 0.3).unwrap());
    assert!(Orbit::from_linear(0.0, 1.0, 0.0).is_err());
    assert!(Orbit::from_linear(-3.0, 1.0, 0.0).is_err());
    assert!(Orbit::from_linear(3.0, f32::NAN, 0.0).is_err());
    assert_eq!(
        ScaleProfile::project3().max_orbit_speed,
        MAX_ORBIT_LINEAR_SPEED
    );
    let s = project3();
    for (b, p) in s.bodies[1..].iter().zip(&SystemSpec::project3().planets) {
        let o = b.orbit.unwrap();
        assert!((o.speed * o.radius - p.orbit_speed).abs() < 1e-6);
        assert!(p.orbit_speed.abs() <= MAX_ORBIT_LINEAR_SPEED);
    }
}

/// Seconds a ship at `speed`, always heading for the planet's current
/// position from `from`, takes to come within `range` of the planet
/// `orbit` starting at `t0`; `None` after `limit` seconds.
fn pursuit(
    orbit: Orbit,
    from: Vector3,
    speed: f32,
    range: f32,
    t0: f32,
    limit: f32,
) -> Option<f32> {
    let dt = 1.0 / 60.0;
    let mut p = from;
    let mut t = 0.0;
    while t < limit {
        let target = orbit.position_at(t0 + t, Vector3::zero());
        let to = target - p;
        if to.length() <= range {
            return Some(t);
        }
        p += to.normalized() * (speed * dt);
        t += dt;
    }
    None
}

#[test]
fn every_planet_reachable_in_cruise_under_60s() {
    let s = project3();
    let spawn = s.ship.position;
    let cruise = s.ship.move_speed;
    assert_eq!(cruise, ScaleProfile::project3().cruise_speed);
    for i in 1..s.bodies.len() {
        let orbit = s.bodies[i].orbit.unwrap();
        // Close enough for `L` (within the local range of the surface).
        let range = s.radius_of(i) + s.flight_scale(i).local_range;
        // Whatever the planet's place on its orbit when the flight starts.
        for t0 in (0..12).map(|k| k as f32 * std::f32::consts::TAU / 12.0 / orbit.speed) {
            let took = pursuit(orbit, spawn, cruise, range, t0, 60.0);
            assert!(took.is_some(), "planet {i} from t0 {t0}");
        }
    }
    // A planet faster than the cruise, chased from behind, is never caught.
    let fast = Orbit::from_linear(36.0, 1.5 * cruise, 0.0).unwrap();
    let behind = fast.position_at(-2.0, Vector3::zero());
    assert_eq!(pursuit(fast, behind, cruise, 6.0, 0.0, 60.0), None);
}

#[test]
fn dt_split_is_reproducible() {
    // The same simulated time reached in steps of 1/30 s or 1/60 s puts
    // every planet in the same place (closed-form orbits).
    let advance = |dt: f32, steps: usize| {
        let mut s = project3();
        for _ in 0..steps {
            s.advance(dt);
            s.update();
        }
        s
    };
    let a = advance(1.0 / 30.0, 90);
    let b = advance(1.0 / 60.0, 180);
    assert!((a.time() - b.time()).abs() < 1e-4);
    for (x, y) in a.bodies.iter().zip(&b.bodies) {
        assert!((x.transform.position - y.transform.position).length() < 1e-3);
    }
    // And the ship flies the same distance either way.
    let fly = |dt: f32, steps: usize| {
        let mut s = project3();
        let input = FlightInput::from_actions(&[SolarAction::Forward]);
        for _ in 0..steps {
            rust_software_renderer::solar::flight::space_step(&mut s.ship, &input, dt);
        }
        s.ship.position
    };
    assert!((fly(1.0 / 30.0, 90) - fly(1.0 / 60.0, 180)).length() < 0.05);
}

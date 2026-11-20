//! Project 3 vehicle routes (C63): closed, deterministic, always over the
//! ground, with an orthonormal heading and up, frozen with the clock.

use raylib::math::Vector3;
use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::SimulationClock;
use rust_software_renderer::solar::attach::path::{Path, PathPose};
use rust_software_renderer::solar::planet::{CloverField, HeartsField, HeightField};

fn fields() -> [HeightField; 2] {
    [
        HeightField::Clover(CloverField::new(7)),
        HeightField::Hearts(HeartsField::new(11)),
    ]
}

fn paths() -> [Path; 3] {
    [
        Path::GreatCircle {
            axis: Vector3::new(0.3, 1.0, -0.2),
            speed: 0.4,
            phase: 1.0,
            altitude: 0.4,
        },
        Path::Latitude {
            lat: 0.5,
            speed: -0.7,
            phase: 0.2,
            altitude: 0.01,
        },
        Path::GreatCircle {
            axis: Vector3::new(1.0, 0.0, 0.0),
            speed: 1.3,
            phase: 0.0,
            altitude: 0.01,
        },
    ]
}

fn close(a: PathPose, b: PathPose, eps: f32) -> bool {
    (a.pos - b.pos).length() < eps
        && (a.tangent - b.tangent).length() < eps
        && (a.up - b.up).length() < eps
}

#[test]
fn closed_period() {
    for field in fields() {
        for path in paths() {
            for t in [0.0, 3.7, 50.0] {
                let a = path.pose(t, &field, SPHERE_RADIUS);
                let b = path.pose(t + path.period(), &field, SPHERE_RADIUS);
                assert!(close(a, b, 1e-3), "{path:?} at {t}");
            }
        }
    }
}

#[test]
fn deterministic() {
    let field = &fields()[1];
    for path in paths() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        path.sample(2.0, 9.0, 40, field, SPHERE_RADIUS, &mut a);
        path.sample(2.0, 9.0, 40, field, SPHERE_RADIUS, &mut b);
        assert_eq!(a, b);
        assert_eq!(a.len(), 40);
        assert_eq!(a[0], path.pose(2.0, field, SPHERE_RADIUS));
        assert!(close(a[39], path.pose(9.0, field, SPHERE_RADIUS), 1e-5));
        // The buffer is reused: cleared, not grown.
        let capacity = a.capacity();
        path.sample(0.0, 1.0, 10, field, SPHERE_RADIUS, &mut a);
        assert_eq!((a.len(), a.capacity()), (10, capacity));
    }
}

#[test]
fn above_terrain_always() {
    for field in fields() {
        for path in paths() {
            for i in 0..2000 {
                let p = path.pose(i as f32 * 0.05, &field, SPHERE_RADIUS);
                let ground = SPHERE_RADIUS * field.surface_radius(p.up);
                let over = (p.pos.length() - ground) / SPHERE_RADIUS;
                assert!(
                    (over - path.altitude()).abs() < 1e-4,
                    "{path:?} at step {i}: {over}"
                );
            }
        }
    }
}

#[test]
fn tangent_unit_and_orthogonal_to_up() {
    for field in fields() {
        for path in paths() {
            let mut poses = Vec::new();
            path.sample(0.0, path.period(), 100, &field, SPHERE_RADIUS, &mut poses);
            for (i, p) in poses.iter().enumerate() {
                assert!((p.tangent.length() - 1.0).abs() < 1e-4, "{i}");
                assert!((p.up.length() - 1.0).abs() < 1e-4, "{i}");
                assert!(p.tangent.dot(p.up).abs() < 1e-4, "{i}");
            }
            // The heading points where the vehicle goes next (over the
            // ground: the climb follows the terrain).
            for w in poses.windows(2) {
                let moved = w[1].pos - w[0].pos;
                let along = (moved - w[0].up * moved.dot(w[0].up)).normalized();
                assert!(along.dot(w[0].tangent) > 0.99, "{path:?}");
            }
        }
    }
}

#[test]
fn paused_time_frozen() {
    let field = &fields()[0];
    let mut clock = SimulationClock::default();
    clock.advance(1.0 / 60.0);
    clock.paused = true;
    let before: Vec<PathPose> = paths()
        .iter()
        .map(|p| p.pose(clock.time, field, SPHERE_RADIUS))
        .collect();
    for _ in 0..120 {
        clock.advance(1.0 / 60.0);
    }
    let after: Vec<PathPose> = paths()
        .iter()
        .map(|p| p.pose(clock.time, field, SPHERE_RADIUS))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn still_route_is_fixed_and_finite() {
    let field = &fields()[1];
    let path = Path::GreatCircle {
        axis: Vector3::new(0.0, 0.0, 1.0),
        speed: 0.0,
        phase: 0.7,
        altitude: 0.3,
    };
    let first = path.pose(0.0, field, SPHERE_RADIUS);
    for t in [1.0, 100.0, 1e6] {
        let p = path.pose(t, field, SPHERE_RADIUS);
        assert_eq!(p, first);
        assert!(p.pos.x.is_finite() && p.tangent.x.is_finite() && p.up.x.is_finite());
    }
    assert!(path.period().is_infinite());
}

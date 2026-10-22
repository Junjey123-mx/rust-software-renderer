use raylib::math::Vector3;
use rust_software_renderer::pipeline::orbit_camera::{
    MAX_ELEVATION, MAX_ZOOM, MIN_ELEVATION, MIN_ZOOM,
};
use rust_software_renderer::pipeline::{OrbitCamera, ViewPreset};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

fn unit(v: Vector3) -> bool {
    close(v.length(), 1.0)
}

#[test]
fn basis_is_orthonormal_for_many_orientations() {
    let mut cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    for i in 0..60 {
        cam.azimuth = -3.1 + i as f32 * 0.105;
        for j in 0..20 {
            cam.elevation = MIN_ELEVATION + j as f32 * (MAX_ELEVATION - MIN_ELEVATION) / 19.0;
            let b = cam.basis();
            assert!(unit(b.right) && unit(b.up) && unit(b.forward), "{cam:?}");
            assert!(
                close(b.right.dot(b.up), 0.0)
                    && close(b.up.dot(b.forward), 0.0)
                    && close(b.right.dot(b.forward), 0.0),
                "{cam:?}"
            );
            // Right-handed: right x up = -forward? forward x up... check cross(right, forward) == up.
            let u = b.right.cross(b.forward);
            assert!(close(u.x, b.up.x) && close(u.y, b.up.y) && close(u.z, b.up.z));
            assert!(
                b.up.y >= 0.0,
                "screen up never flips below the horizon: {cam:?}"
            );
        }
    }
}

#[test]
fn target_maps_to_the_view_origin_and_depth_orders_points() {
    let target = Vector3::new(1.0, -2.0, 3.0);
    let cam = OrbitCamera::new(target, ViewPreset::Oblique);
    let o = cam.world_to_view(target);
    assert!(close(o.x, 0.0) && close(o.y, 0.0) && close(o.z, 0.0));
    // A point towards the eye has smaller depth than one behind the target.
    let eye_side = target + cam.eye_direction() * 2.0;
    let far_side = target - cam.eye_direction() * 2.0;
    assert!(cam.world_to_view(eye_side).z < cam.world_to_view(far_side).z);
    assert!(close(cam.world_to_view(eye_side).z, -2.0));
}

#[test]
fn top_view_looks_down_and_front_and_side_views_are_level() {
    let top = OrbitCamera::new(Vector3::zero(), ViewPreset::Top);
    let above = top.world_to_view(Vector3::new(0.0, 5.0, 0.0));
    assert!(above.z < -4.9, "a point above is nearer the top camera");
    // Width along X stays horizontal on screen in the top view.
    let wing = top.world_to_view(Vector3::new(3.0, 0.0, 0.0));
    assert!(close(wing.x.abs(), 3.0) && close(wing.y, 0.0));

    let front = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let nose = front.world_to_view(Vector3::new(0.0, 0.0, -4.0));
    assert!(nose.z < 0.0, "nose is nearest to the front camera");
    let up = front.world_to_view(Vector3::new(0.0, 1.0, 0.0));
    assert!(close(up.y, 1.0) && close(up.x, 0.0));

    let side = OrbitCamera::new(Vector3::zero(), ViewPreset::Side);
    let tip = side.world_to_view(Vector3::new(4.0, 0.0, 0.0));
    assert!(
        tip.z < 0.0,
        "positive X wing tip is nearest to the side camera"
    );
    let nose = side.world_to_view(Vector3::new(0.0, 0.0, -4.0));
    assert!(
        close(nose.x.abs(), 4.0) && close(nose.y, 0.0),
        "length is horizontal on screen"
    );
}

#[test]
fn elevation_reveals_height_and_azimuth_changes_silhouette() {
    let mut cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let tall = Vector3::new(0.0, 1.0, 0.0);
    let level = cam.world_to_view(tall).y;
    cam.tilt(0.8);
    let tilted = cam.world_to_view(tall).y;
    assert!(
        tilted < level && tilted > 0.0,
        "height foreshortens when looking from above"
    );
    let nose = Vector3::new(0.0, 0.0, -4.0);
    cam.set_preset(ViewPreset::Front);
    let x_front = cam.world_to_view(nose).x;
    cam.orbit(std::f32::consts::FRAC_PI_2);
    let x_rotated = cam.world_to_view(nose).x;
    assert!(close(x_front, 0.0) && x_rotated.abs() > 3.9);
}

#[test]
fn clamps_wraps_and_reset_are_exact() {
    let mut cam = OrbitCamera::new(Vector3::new(0.5, 0.5, 0.5), ViewPreset::Side);
    let initial = cam;
    cam.tilt(10.0);
    assert_eq!(cam.elevation, MAX_ELEVATION);
    cam.tilt(-20.0);
    assert_eq!(cam.elevation, MIN_ELEVATION);
    cam.zoom_by(100.0);
    assert_eq!(cam.zoom, MAX_ZOOM);
    cam.zoom_by(0.0001);
    assert_eq!(cam.zoom, MIN_ZOOM);
    for _ in 0..1000 {
        cam.orbit(0.7);
        assert!(
            cam.azimuth > -std::f32::consts::PI - 1e-6
                && cam.azimuth <= std::f32::consts::PI + 1e-6
        );
        assert!(cam.basis().forward.x.is_finite());
    }
    cam.reset();
    assert_eq!(cam, initial);
    assert_eq!(cam.preset(), ViewPreset::Side);
}

#[test]
fn projections_stay_finite_for_extreme_inputs() {
    let cam = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    for p in [
        Vector3::new(1e6, -1e6, 1e6),
        Vector3::new(-1e-6, 1e-6, 0.0),
        Vector3::zero(),
    ] {
        let v = cam.world_to_view(p);
        assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
    }
    assert!("top".parse::<ViewPreset>().unwrap() == ViewPreset::Top);
    assert!("diagonal".parse::<ViewPreset>().is_err());
}

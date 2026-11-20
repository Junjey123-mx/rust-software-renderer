//! Project 3 relief shading: the height field's normal, finite everywhere
//! (poles included), continuous across the mesh's edges, exactly the
//! sphere's shading with the bump off and a different lighting with it on.

use raylib::math::Vector3;
use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    FragmentView, OrbitCamera, OrbitView, ShadedOptions, SurfaceSample, Transform, ViewPreset,
    ViewProjection, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::instance::prepare_instance;
use rust_software_renderer::solar::planet::field::NORMAL_EPS;
use rust_software_renderer::solar::planet::icosphere::icosphere;
use rust_software_renderer::solar::planet::material::relief_normal;
use rust_software_renderer::solar::planet::{HeightField, PreparedPlanet, TestField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{BodyInstance, FrameContext, SolarMeshes, SolarScene};

fn field() -> HeightField {
    HeightField::Test(TestField {
        amplitude: 0.04,
        frequency: 6.0,
        sea_level: Some(0.0),
    })
}

/// Directions over the whole sphere, the six poles of the axes included.
fn directions() -> Vec<Vector3> {
    let mut d: Vec<Vector3> = (0..2000)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 2000.0;
            let r = (1.0 - y * y).sqrt();
            let a = i as f32 * 2.399_963;
            Vector3::new(r * a.cos(), y, r * a.sin())
        })
        .collect();
    for axis in [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ] {
        d.push(axis);
        d.push(-axis);
    }
    d
}

fn bits(v: Vector3) -> (u32, u32, u32) {
    (v.x.to_bits(), v.y.to_bits(), v.z.to_bits())
}

#[test]
fn no_nan_normals() {
    for n in directions() {
        let normal = relief_normal(&field(), n * 0.9, 1.0);
        assert!(
            normal.x.is_finite() && normal.y.is_finite() && normal.z.is_finite(),
            "{n:?}"
        );
        assert!((normal.length() - 1.0).abs() < 1e-4, "{n:?}");
        assert!(normal.dot(n) > 0.0, "outward at {n:?}");
        let half = relief_normal(&field(), n, 0.5);
        assert!((half.length() - 1.0).abs() < 1e-4);
    }
    // Degenerate positions do not produce NaN either.
    let zero = relief_normal(&field(), Vector3::zero(), 1.0);
    assert!(zero.length() > 0.99);
    // A zero, negative or NaN step is refused; at the poles it is finite.
    for eps in [0.0, -1e-3, f32::NAN, f32::INFINITY] {
        assert!(
            field()
                .try_normal(Vector3::new(0.0, 1.0, 0.0), eps)
                .is_err()
        );
    }
    for pole in [Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, -1.0, 0.0)] {
        let n = field().try_normal(pole, NORMAL_EPS).unwrap();
        assert!(n.length().is_finite() && (n.length() - 1.0).abs() < 1e-4);
    }
}

fn prepared_and_samples() -> (
    rust_software_renderer::celestial::PreparedShader,
    Transform,
    Vec<SurfaceSample>,
    FragmentView,
) {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(160, 120, 0.1),
    )
    .unwrap();
    let frame = FrameContext {
        projection: ViewProjection::Orbit(view),
        time: 3.7,
        light_world: Vector3::new(0.3, 0.8, -0.5).normalized(),
        mode: TriangleMode::FilledScanline,
        options: ShadedOptions::default(),
    };
    let transform = Transform {
        position: Vector3::new(1.0, 0.2, -2.0),
        yaw: 0.9,
        ..Transform::default()
    };
    let instance = BodyInstance::new(Body::CrimsonEntrance, transform, 0.4);
    let samples = directions()
        .iter()
        .step_by(7)
        .enumerate()
        .map(|(k, &n)| SurfaceSample {
            x: k as i32,
            y: 0,
            depth: 1.0,
            object_position: n * 0.866 * field().surface_radius(n),
            normal: n,
            face_normal: n,
        })
        .collect();
    let v = FragmentView {
        view_direction: Vector3::new(0.1, 0.3, 1.0).normalized(),
    };
    (prepare_instance(&instance, &frame), transform, samples, v)
}

#[test]
fn bump_off_equals_radial() {
    for n in directions() {
        let p = n * 0.93;
        assert_eq!(bits(relief_normal(&field(), p, 0.0)), bits(p / p.length()));
    }
    let (base, transform, samples, v) = prepared_and_samples();
    let lab6 = PreparedPlanet::Lab6(base.clone());
    for bump in [0.0, -1.0, f32::NAN] {
        let off = PreparedPlanet::Relief {
            base: base.clone(),
            field: field(),
            bump,
        };
        for s in &samples {
            assert_eq!(
                bits(off.shade(&transform, s, &v)),
                bits(lab6.shade(&transform, s, &v))
            );
        }
    }
}

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(320, 240, 0.1),
    )
    .unwrap();
    let mut s = SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap();
    s.set_relief(1, field()).unwrap();
    let p = s.bodies[1].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, 2.0 * s.radius_of(1));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

fn frame(s: &SolarScene) -> Vec<u8> {
    let mut fb = Framebuffer::new(320, 240);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb.to_rgba_bytes()
}

#[test]
fn relief_lighting_differs_from_radial() {
    let (base, transform, samples, v) = prepared_and_samples();
    let lab6 = PreparedPlanet::Lab6(base.clone());
    let on = PreparedPlanet::Relief {
        base,
        field: field(),
        bump: 1.0,
    };
    let differing = samples
        .iter()
        .filter(|s| (on.shade(&transform, s, &v) - lab6.shade(&transform, s, &v)).length() > 1e-3)
        .count();
    // The night side is dark either way: a third of all samples is plenty.
    assert!(
        differing * 3 > samples.len(),
        "{differing}/{}",
        samples.len()
    );
    // In the scene: the same relief mesh, bump on and off.
    let bumped = scene();
    let mut flat = bumped.clone();
    flat.relief[1].as_mut().unwrap().bump = 0.0;
    let (a, b) = (frame(&bumped), frame(&flat));
    // `P3_DUMP_DIR=<dir>` writes both frames as PPM, to look for banding.
    if let Some(dir) = std::env::var_os("P3_DUMP_DIR") {
        for (name, rgba) in [("bump_on", &a), ("bump_off", &b)] {
            let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
            let path = std::path::Path::new(&dir).join(format!("p3_relief_{name}.ppm"));
            std::fs::write(path, [b"P6 320 240 255\n".as_slice(), &rgb].concat()).unwrap();
        }
    }
    let changed = a.chunks(4).zip(b.chunks(4)).filter(|(x, y)| x != y).count();
    assert!(changed > 500, "{changed} pixels changed");
}

#[test]
fn normals_continuous_across_triangle_edges() {
    // Either side of every edge of the level 3 icosphere, 2e-4 rad apart:
    // the shading normal is per fragment, from the field, so it does not
    // jump where the triangles meet (no faceting).
    let mesh = icosphere(3).unwrap();
    let mut worst = 1.0f32;
    for tri in &mesh.indices {
        for k in 0..3 {
            let a = mesh.positions[tri[k] as usize];
            let b = mesh.positions[tri[(k + 1) % 3] as usize];
            let mid = (a + b).normalized();
            let across = (b - a).cross(mid).normalized() * 1e-4;
            let left = relief_normal(&field(), mid + across, 1.0);
            let right = relief_normal(&field(), mid - across, 1.0);
            worst = worst.min(left.dot(right));
        }
    }
    assert!(worst > 0.9999, "{worst}");
}

//! Project 3 Hearts pyramid: a stepped neon pyramid about 0.45 radii tall,
//! upright on a plateau, that widens the planet's effective radius and
//! shows from afar and up close.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::landmark::{Landmark, PYRAMID_HEIGHT, nearest_plateau};
use rust_software_renderer::solar::attach::prop_meshes::pyramid_mesh;
use rust_software_renderer::solar::attach::{Attachment, AttachmentKind, AttachmentSpec};
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::field::NORMAL_EPS;
use rust_software_renderer::solar::planet::{HeartsField, HeightField};
use rust_software_renderer::solar::system::layout::effective_radius;
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

fn field() -> HeightField {
    HeightField::Hearts(HeartsField::new(11))
}

fn pyramid() -> Landmark {
    Landmark::pyramid(&field(), Vector3::new(0.3, 0.8, 0.5), SPHERE_RADIUS).unwrap()
}

#[test]
fn height_about_045_r() {
    let l = pyramid();
    let ground = SPHERE_RADIUS * field().surface_radius(l.dir);
    let height = (l.top_local.length() - ground) / SPHERE_RADIUS;
    // Its whole height is 0.45 R, a few percent of it sunk in the ground.
    assert!(
        height > 0.9 * PYRAMID_HEIGHT && height <= PYRAMID_HEIGHT,
        "{height}"
    );
    let (body, edges) = pyramid_mesh(5);
    assert_eq!(body.indices.len() / 3, 5 * 12, "five closed steps");
    assert!(!edges.indices.is_empty());
    assert_eq!(pyramid_mesh(0), pyramid_mesh(2), "at least two steps");
}

#[test]
fn upright_on_surface_normal() {
    let l = pyramid();
    let f = field();
    assert!(f.is_plateau(l.dir));
    assert!((l.up - f.normal(l.dir, NORMAL_EPS)).length() < 1e-5);
    let foot = l.dir * (SPHERE_RADIUS * f.surface_radius(l.dir));
    let axis = (l.top_local - foot).normalized();
    assert!(axis.dot(l.up) > 0.9999, "{}", axis.dot(l.up));
}

#[test]
fn extent_updates_effective_radius() {
    let spec = SystemSpec::project3();
    let planet = &spec.planets[0];
    let l = pyramid();
    let bare = effective_radius(planet, &[]).unwrap();
    let with = effective_radius(planet, &[Attachment::Landmark(&l)]).unwrap();
    assert!(with > bare * 1.4, "{bare} -> {with}");
    assert!((Attachment::Landmark(&l).extent() - l.reach / SPHERE_RADIUS).abs() < 1e-6);
}

#[test]
fn canyon_direction_moves_to_nearest_plateau() {
    let f = field();
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    let canyon = (0..2000)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 2000.0;
            let r = (1.0 - y * y).sqrt();
            Vector3::new(
                r * (golden * i as f32).cos(),
                y,
                r * (golden * i as f32).sin(),
            )
        })
        .find(|&n| !f.is_plateau(n))
        .unwrap();
    let l = Landmark::pyramid(&f, canyon, SPHERE_RADIUS).unwrap();
    assert!(f.is_plateau(l.dir));
    assert!(l.dir.dot(canyon) > 0.6f32.cos(), "it stays close");
    assert_eq!(nearest_plateau(&f, l.dir), Some(l.dir));
    // No terraces anywhere: nowhere to stand.
    assert!(Landmark::pyramid(&HeightField::Flat, canyon, SPHERE_RADIUS).is_err());
}

#[test]
fn visible_far_and_near() {
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
    let l = pyramid();
    let dir = l.dir;
    s.landmark = Some(l);
    s.attachments.push(AttachmentSpec {
        kind: AttachmentKind::Landmark,
        host: PlanetId("entrance"),
    });
    for d in [2.5, 20.0] {
        // Ship on the pyramid's side, looking at the planet.
        let t = s.bodies[1].transform;
        let out = t.to_world_dir(dir).normalized();
        s.ship.position = t.position + out * (d * s.radius_of(1));
        s.ship.forward = -out;
        s.ship.up = out.cross(Vector3::new(1.0, 0.0, 0.0)).normalized();
        s.ship.renormalize();
        let draws = s.effect_draws(320, 240, 1);
        let pyramid_draws = draws
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Prop(p) if p.color == rust_software_renderer::solar::attach::props::PropPart::BuildingBody.color()))
            .count();
        assert_eq!(pyramid_draws, 1, "distance {d}");
        // From the far side it is behind the planet.
        s.ship.position = t.position - out * (d * s.radius_of(1));
        s.ship.forward = out;
        let hidden = s
            .effect_draws(320, 240, 1)
            .iter()
            .filter(|d| matches!(d.shader, EffectShader::Prop(_)))
            .count();
        assert_eq!(hidden, 0, "distance {d}");
    }
}

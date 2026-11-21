//! Ace Crystal (`planet::ace`): seeded crystal clusters standing outward on
//! the ground, a cyan-white palette, the relief bound kept and a scene with
//! the planet that draws its crystals. `SystemSpec::project3` keeps its
//! five planets; these tests add `ace_planet` themselves.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::attach::props::{ScatterRule, frame_matrix, scatter};
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::flight::THRUST_ACCEL;
use rust_software_renderer::solar::physics::design_mu;
use rust_software_renderer::solar::planet::HeightField;
use rust_software_renderer::solar::planet::ace::{
    ACE_RELIEF_BOUND, AceField, AceParams, CLUSTER_HEIGHT, CRYSTAL_VARIANTS, CrystalShader,
    PreparedAce, crystal_cluster,
};
use rust_software_renderer::solar::scene::CameraMode;
use rust_software_renderer::solar::system::{PlanetId, SystemSpec, ace_planet};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};

/// The Project 3 system with Ace Crystal (radius 1.5) beyond the Joker.
fn spec() -> SystemSpec {
    let mut spec = SystemSpec::project3();
    let mut ace = ace_planet();
    ace.scale = 1.5 / SPHERE_RADIUS;
    ace.orbit_radius = 57.0;
    ace.orbit_speed = 0.65;
    ace.mu = design_mu(1.5, THRUST_ACCEL, 3.0);
    spec.planets.push(ace);
    spec
}

fn scene(spec: &SystemSpec, w: u32, h: u32) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        60.0,
        Viewport::new(w, h, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        spec,
    )
    .unwrap()
}

/// Unit directions spread over the sphere (Fibonacci lattice).
fn directions(count: u32) -> impl Iterator<Item = Vector3> {
    (0..count).map(move |i| {
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
        let r = (1.0 - y * y).sqrt();
        let a = i as f32 * 2.399_963;
        Vector3::new(r * a.cos(), y, r * a.sin())
    })
}

#[test]
fn project3_keeps_five_planets() {
    let ids: Vec<_> = SystemSpec::project3()
        .planets
        .iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(ids.len(), 5);
    assert!(!ids.contains(&PlanetId("ace")));
}

#[test]
fn crystals_follow_the_seed() {
    let field = HeightField::Ace(AceField::new(41));
    let a = scatter(&field, &ScatterRule::CRYSTALS, 41);
    let b = scatter(&field, &ScatterRule::CRYSTALS, 41);
    let c = scatter(&field, &ScatterRule::CRYSTALS, 42);
    assert!(a.len() > 100, "{} crystals", a.len());
    assert_eq!(a, b);
    assert_ne!(a, c);
    // The scene bakes the same sites twice, and big spires among them.
    let spec = spec();
    let (s1, s2) = (scene(&spec, 80, 60), scene(&spec, 80, 60));
    let sites = |s: &SolarScene| s.crystals.as_ref().unwrap().sites.clone();
    assert_eq!(sites(&s1), sites(&s2));
    assert!(sites(&s1).len() > 100);
}

#[test]
fn crystals_stand_outward_on_the_ground() {
    let field = HeightField::Ace(AceField::new(41));
    for pose in scatter(&field, &ScatterRule::CRYSTALS, 41) {
        assert!(pose.up.dot(pose.dir) > 0.4, "leans too far: {pose:?}");
        let map = frame_matrix(&pose, &field, 1.0, 0.11);
        let (foot, tip) = (map(Vector3::zero()), map(Vector3::new(0.0, 1.0, 0.0)));
        assert!((foot.length() - field.surface_radius(pose.dir)).abs() < 1e-4);
        assert!(tip.length() > foot.length() + 0.05, "tip not above foot");
    }
    // Every cluster: tips up to its height, sunk a little into the ground.
    for v in 0..CRYSTAL_VARIANTS {
        let mesh = crystal_cluster(v);
        let ys = mesh.vertices.iter().map(|p| p.y);
        let (lo, hi) = ys.fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(y), b.max(y)));
        assert!(
            lo >= -0.15 && hi <= CLUSTER_HEIGHT && hi > 0.9,
            "{lo}..{hi}"
        );
        assert_eq!(mesh.indices.len() % 54, 0, "18 triangles per crystal");
    }
}

#[test]
fn relief_and_crystals_stay_inside_the_bound() {
    let field = AceField::new(41);
    let mut highest = 0.0f32;
    for n in directions(4000) {
        let h = field.height(n);
        assert!((0.0..=field.max_height()).contains(&h), "{h}");
        highest = highest.max(h);
    }
    assert!(highest > 0.5 * field.max_height(), "shards rise: {highest}");
    let s = scene(&spec(), 80, 60);
    let crystals = s.crystals.as_ref().unwrap();
    let bound = SPHERE_RADIUS * (1.0 + ACE_RELIEF_BOUND);
    let mut top = 0.0f32;
    for layer in &crystals.layers {
        for sector in &layer.sectors {
            for v in sector.mesh.iter() {
                top = top.max(v.position.length());
            }
        }
    }
    assert!(top <= bound, "{top} > {bound}");
    // The spires are big: at least a fifth of a radius over the ground.
    assert!(top > SPHERE_RADIUS * 1.25, "{top}");
}

#[test]
fn palette_is_cyan_and_white() {
    let light = Vector3::new(0.3, 0.8, 0.5).normalized();
    let ground = PreparedAce {
        field: AceField::new(41),
        params: AceParams::default(),
        light,
        time: 3.0,
    };
    let crystal = CrystalShader {
        light,
        transform: Transform::new(Vector3::zero(), 1.0),
        time: 3.0,
        params: AceParams::default(),
    };
    let (mut sum_ground, mut sum_crystal, mut lit) = (Vector3::zero(), Vector3::zero(), 0);
    for n in directions(3000) {
        if n.dot(light) < 0.3 {
            continue;
        }
        lit += 1;
        sum_ground += ground.shade(n * 1.02, n, n);
        sum_crystal += crystal.shade(n * 1.1, n, n);
    }
    for (name, sum) in [("ground", sum_ground), ("crystal", sum_crystal)] {
        let mean = sum / lit as f32;
        assert!(
            mean.z > mean.x && mean.y > mean.x,
            "{name} not cyan: {mean:?}"
        );
        assert!(mean.z > 0.6, "{name} too dark: {mean:?}");
    }
}

#[test]
fn ace_scene_draws_bright_crystals() {
    let (w, h) = (400, 300);
    let mut s = scene(&spec(), w, h);
    s.camera_mode = CameraMode::Perspective;
    let i = s.body_index(PlanetId("ace")).unwrap();
    let t = s.bodies[i].transform;
    let r = s.radius_of(i);
    let sun = (s.center - t.position).normalized();
    s.ship.position = t.position + sun * (2.6 * r);
    s.ship.forward = -sun;
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s.ship.renormalize();
    let draws = s.effect_draws(w, h, 1);
    assert!(
        draws
            .iter()
            .any(|d| matches!(d.shader, EffectShader::Crystal(_))),
        "no crystal draw"
    );
    let mut fb = Framebuffer::new(w, h);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    let icy = fb
        .to_rgba_bytes()
        .chunks(4)
        .filter(|p| p[2] > 150 && p[1] > p[0] && p[2] > p[0])
        .count();
    assert!(icy > (w * h / 20) as usize, "{icy} icy pixels");
}

#[test]
fn set_reaches_the_ace_material() {
    let mut spec = spec();
    assert_eq!(spec.apply_set("ace.glow", 0.5), Ok(true));
    assert!(spec.apply_set("ace.glow", 5.0).is_err());
}

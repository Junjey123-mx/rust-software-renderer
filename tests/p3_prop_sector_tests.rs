//! Project 3 baked props: hundreds of trees and rocks drawn as a handful
//! of sector meshes that turn with their planet, with the far hemisphere
//! and small planets left out.

use raylib::math::Vector3;
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::props::{
    PropLayers, SECTOR_COUNT, ScatterRule, bake, beyond_horizon, scatter,
};
use rust_software_renderer::solar::attach::{AttachmentKind, AttachmentSpec};
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::{CloverField, HeightField};
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};
use std::time::Instant;

const W: u32 = 800;
const H: u32 = 600;

fn field() -> HeightField {
    HeightField::Clover(CloverField::new(7))
}

fn layers() -> (PropLayers, usize) {
    let mut poses = scatter(&field(), &ScatterRule::TREES, 7);
    poses.extend(scatter(&field(), &ScatterRule::ROCKS, 8));
    let count = poses.len();
    (
        bake(&poses, &field(), SPHERE_RADIUS, 0.05 * SPHERE_RADIUS, 7),
        count,
    )
}

/// The project 3 system with the clover relief and its props on body 1,
/// the ship `d` radii from its center looking at it.
fn scene(d: f32) -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
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
    s.props = Some(layers().0);
    s.attachments.push(AttachmentSpec {
        kind: AttachmentKind::Props,
        host: PlanetId(s.bodies[1].body().name()),
    });
    let p = s.bodies[1].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(1));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

fn prop_draws(s: &SolarScene) -> usize {
    // Body 1's only: since C61 the Hearts city draws with the prop shader
    // too.
    let host = s.bodies[1].transform.matrix();
    s.effect_draws(W, H, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Prop(_)) && d.model.model_matrix == host)
        .count()
}

#[test]
fn draw_count_is_sectors_not_props() {
    let start = Instant::now();
    let (layers, props) = layers();
    let baked = start.elapsed();
    println!(
        "{props} props in {} sector meshes, baked in {baked:?}",
        layers.sector_count()
    );
    assert!(props >= 150, "{props}");
    assert!(layers.sector_count() <= 3 * SECTOR_COUNT);
    let baked_props: usize = layers
        .layers
        .iter()
        .flat_map(|l| &l.sectors)
        .map(|s| s.props)
        .sum();
    // Trees are counted in their trunk and their canopy layers.
    assert!(baked_props >= props);
    let s = scene(2.5);
    let draws = prop_draws(&s);
    assert!(draws >= 1 && draws <= layers.sector_count(), "{draws}");
    assert!(draws * 5 < props, "{draws} draws for {props} props");
}

#[test]
fn layer_rotates_with_host() {
    let mut s = scene(2.5);
    for yaw in [0.0, 1.3] {
        s.bodies[1].transform.yaw = yaw;
        let host = s.bodies[1].transform.matrix();
        for d in s.effect_draws(W, H, 1) {
            if matches!(d.shader, EffectShader::Prop(_)) {
                assert_eq!(d.model.model_matrix, host, "yaw {yaw}");
            }
        }
    }
}

#[test]
fn back_hemisphere_sectors_culled() {
    let s = scene(2.5);
    let props = s.props.as_ref().unwrap();
    let projection = s.projection();
    // C52 replaced the facing cosine (-0.3 at the sector's center, which
    // also dropped sectors whose near edge still showed in a low flight)
    // with the horizon test of `patch_hidden`.
    let facing = props
        .layers
        .iter()
        .flat_map(|l| &l.sectors)
        .filter(|sec| !s.patch_hidden(1, &projection, sec.bound_center, sec.bound_radius))
        .count();
    let draws = prop_draws(&s);
    assert!(draws <= facing, "{draws} > {facing}");
    assert!(facing < props.sector_count(), "the far side is skipped");
    // An empty layer set draws nothing; a far planet (under 60 px) neither.
    let mut empty = scene(2.5);
    empty.props = Some(bake(&[], &field(), SPHERE_RADIUS, 0.05, 1));
    assert_eq!(prop_draws(&empty), 0);
    assert_eq!(prop_draws(&scene(60.0)), 0);
}

#[test]
fn world_poses_follow_spin() {
    let s = scene(2.5);
    let props = s.props.as_ref().unwrap();
    let mut t = s.bodies[1].transform;
    let v = props.layers[0].sectors[0].mesh[0].position;
    t.yaw = 0.2;
    let a = t.position + t.to_world_dir(v) * t.scale - t.position;
    t.yaw = 0.2 + 0.9;
    let b = t.position + t.to_world_dir(v) * t.scale - t.position;
    // Same distance and height, turned by the spin about the host axis.
    assert!((a.length() - b.length()).abs() < 1e-4);
    assert!((a.y - b.y).abs() < 1e-4);
    let turned = (a.x * b.x + a.z * b.z) / (a.x.hypot(a.z) * b.x.hypot(b.z));
    assert!((turned - 0.9f32.cos()).abs() < 1e-3);
    // Every baked prop stands on the planet: none inside the sea floor,
    // none floating far above the relief.
    for layer in &props.layers {
        for sector in &layer.sectors {
            for vtx in sector.mesh.iter() {
                let r = vtx.position.length() / SPHERE_RADIUS;
                assert!(r > 1.0 && r < 1.0 + 0.2, "{r}");
            }
        }
    }
}

/// `P3_DUMP_DIR=<dir>` writes the clover planet with its props from about
/// 1.5 and 1.25 radii, facing clover 0, to look at the trees and rocks.
#[test]
fn props_frame_dump() {
    use rust_software_renderer::pipeline::ShadedOptions;
    use rust_software_renderer::rasterization::TriangleMode;
    use rust_software_renderer::rendering::Framebuffer;
    let Some(dir) = std::env::var_os("P3_DUMP_DIR") else {
        return;
    };
    let mut s = scene(2.5);
    s.bodies[1].transform.yaw = 0.0;
    let HeightField::Clover(f) = field() else {
        unreachable!()
    };
    let c = f.centers[0];
    for (k, d) in [(0, 1.5f32), (1, 1.25)] {
        let p = s.bodies[1].transform.position;
        let side = c.cross(Vector3::new(0.3, 1.0, 0.1)).normalized();
        s.ship.position = p + (c * d + side * 0.25) * s.radius_of(1);
        s.ship.forward = (p + c * s.radius_of(1) - s.ship.position).normalized();
        s.ship.up = s.ship.forward.cross(side).normalized();
        let mut fb = Framebuffer::new(W, H);
        s.render_frame(
            &mut fb,
            TriangleMode::FilledScanline,
            ShadedOptions::default(),
        )
        .unwrap();
        let rgb: Vec<u8> = fb
            .to_rgba_bytes()
            .chunks(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let path = std::path::Path::new(&dir).join(format!("p3_props_{k}.ppm"));
        std::fs::write(
            path,
            [format!("P6 {W} {H} 255\n").as_bytes(), &rgb].concat(),
        )
        .unwrap();
    }
}

#[test]
fn horizon_keeps_near_edges() {
    let up = Vector3::new(0.0, 1.0, 0.0);
    let at = |deg: f32| {
        let a = deg.to_radians();
        Vector3::new(a.sin(), a.cos(), 0.0)
    };
    let cell = 37f32.to_radians();
    // Low flight (0.12 radii up): a sector centered 50 degrees away has its
    // near edge 13 degrees away, well inside the 27 degree horizon.
    assert!(!beyond_horizon(up, 1.12, at(50.0), cell, 1.1, 1.0));
    // Opposite side: hidden.
    assert!(beyond_horizon(up, 1.12, at(150.0), cell, 1.1, 1.0));
    // A tall model just past the horizon still shows.
    assert!(!beyond_horizon(up, 1.12, at(30.0), 0.0, 1.1, 1.0));
    assert!(beyond_horizon(up, 1.12, at(30.0), 0.0, 1.0, 1.0));
    // Under the orthographic camera the horizon is at 90 degrees.
    assert!(!beyond_horizon(up, f32::INFINITY, at(89.0), 0.0, 1.0, 1.0));
    assert!(beyond_horizon(up, f32::INFINITY, at(91.0), 0.0, 1.0, 1.0));
    // An eye inside the occluder (or not a number) hides nothing.
    assert!(!beyond_horizon(up, 0.9, at(170.0), 0.0, 1.0, 1.0));
    assert!(!beyond_horizon(up, f32::NAN, at(170.0), 0.0, 1.0, 1.0));
}

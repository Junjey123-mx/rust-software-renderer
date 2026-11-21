//! Project 3 surface models (C62): four distinct kinds of procedural models
//! stand on the planets' ground and turn with them: trees and rocks on the
//! clover, buildings and the pyramid on the hearts planet. Variants of one
//! kind (three trees, three towers) count once; the royal crown rides the
//! star and the diamond cards orbit House of Cards, so neither counts.

use raylib::math::Vector3;
use rust_software_renderer::geometry::{Mesh, Vertex};
use rust_software_renderer::pipeline::{OrbitCamera, OrbitView, ViewPreset, Viewport};
use rust_software_renderer::solar::attach::AttachmentKind;
use rust_software_renderer::solar::attach::prop_meshes::{
    pyramid_mesh, rock_mesh, tower_mesh, tree_mesh,
};
use rust_software_renderer::solar::attach::props::PropPart;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::planet::HeightField;
use rust_software_renderer::solar::system::{PlanetId, SystemSpec};
use rust_software_renderer::solar::{SPHERE_RADIUS, SolarMeshes, SolarScene};
use std::collections::BTreeSet;

const W: u32 = 800;
const H: u32 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Tree,
    Rock,
    Building,
    Pyramid,
    /// Ace Crystal's clusters (not in the Project 3 system).
    Crystal,
}

fn kind_of(part: PropPart) -> Kind {
    match part {
        PropPart::TreeTrunk | PropPart::TreeCanopy => Kind::Tree,
        PropPart::Rock => Kind::Rock,
        PropPart::BuildingBody | PropPart::BuildingEdge => Kind::Building,
        PropPart::Crystal => Kind::Crystal,
    }
}

fn scene() -> SolarScene {
    let view = OrbitView::for_world(
        OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique),
        8.0,
        Viewport::new(W, H, 0.1),
    )
    .unwrap();
    SolarScene::scaled_from_spec(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 1.0, 0.0),
        &SystemSpec::project3(),
    )
    .unwrap()
}

fn index(s: &SolarScene, id: &'static str) -> usize {
    s.body_index(PlanetId(id)).unwrap()
}

fn field(s: &SolarScene, i: usize) -> HeightField {
    s.relief_of(i).expect("a planet with a surface").field
}

/// Height of each vertex over the ground right below it, planet radii.
fn heights(field: &HeightField, verts: &[Vertex]) -> Vec<f32> {
    verts
        .iter()
        .map(|v| {
            v.position.length() / SPHERE_RADIUS - field.surface_radius(v.position.normalized())
        })
        .collect()
}

/// The models of `verts` stand on the surface of `field`: no vertex sinks
/// more than `sink` under the ground or rises over `top`, and some are
/// within `sink` of it (a flat base over the curved ground lifts its
/// corners).
fn stands_on(field: &HeightField, verts: &[Vertex], sink: f32, top: f32) -> bool {
    let h = heights(field, verts);
    h.iter().all(|&h| h > -sink && h < top) && h.iter().any(|&h| h.abs() < sink)
}

/// Every kind with its host and its baked vertices (host mesh space).
fn placed(s: &SolarScene) -> Vec<(Kind, usize, Vec<Vertex>)> {
    let (clover, hearts) = (index(s, "clover"), index(s, "hearts"));
    let mut out = Vec::new();
    for (host, layers) in [(clover, s.props.as_ref()), (hearts, s.city.as_ref())] {
        for layer in &layers.unwrap().layers {
            let verts = layer
                .sectors
                .iter()
                .flat_map(|sec| sec.mesh.iter().copied());
            out.push((kind_of(layer.part), host, verts.collect()));
        }
    }
    let l = s.landmark.as_ref().unwrap();
    let pyramid = l.mesh.iter().chain(l.edges.iter()).copied().collect();
    out.push((Kind::Pyramid, hearts, pyramid));
    out
}

/// Triangles and height / width of one model.
fn shape(meshes: &[Mesh]) -> (usize, f32) {
    let points = meshes.iter().flat_map(|m| m.vertices.iter());
    let (lo, hi) = points.fold(
        (Vector3::one() * f32::MAX, Vector3::one() * f32::MIN),
        |(lo, hi), p| {
            (
                Vector3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                Vector3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        },
    );
    let triangles = meshes.iter().map(|m| m.indices.len() / 3).sum();
    let size = hi - lo;
    (triangles, size.y / size.x.max(size.z))
}

#[test]
fn four_distinct_kinds_on_planets() {
    let s = scene();
    let placed = placed(&s);
    let kinds: BTreeSet<Kind> = placed.iter().map(|p| p.0).collect();
    assert_eq!(kinds.len(), 4, "{kinds:?}");
    // Each stands on its planet's ground: props and buildings sink a
    // little on slopes and terrace edges, the pyramid by design (6 % of its
    // 0.45 R plus the terrace step under its base).
    for (kind, host, verts) in &placed {
        let f = field(&s, *host);
        let (sink, top) = match kind {
            Kind::Pyramid => (0.1, 0.5),
            _ => (0.03, 0.12),
        };
        let h = heights(&f, verts);
        let (lo, hi) = h
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
        assert!(stands_on(&f, verts, sink, top), "{kind:?}: {lo}..{hi}");
    }
    // Negative case: the same pyramid lifted 0.3 R off the ground is not
    // on the surface.
    let (_, hearts, pyramid) = placed.last().unwrap();
    let lifted: Vec<Vertex> = pyramid
        .iter()
        .map(|v| {
            let mut v = *v;
            v.position += v.position.normalized() * (0.3 * SPHERE_RADIUS);
            v
        })
        .collect();
    assert!(!stands_on(&field(&s, *hearts), &lifted, 0.1, 0.5));
    // Four different shapes, not variants of one: triangle count and
    // height / width differ pairwise.
    let shapes = [
        shape(&[tree_mesh(1)]),
        shape(&[rock_mesh(7)]),
        {
            let (body, edges) = tower_mesh(0);
            shape(&[body, edges])
        },
        {
            let (body, edges) = pyramid_mesh(5);
            shape(&[body, edges])
        },
    ];
    for (i, a) in shapes.iter().enumerate() {
        for b in &shapes[i + 1..] {
            assert_ne!(a.0, b.0, "{shapes:?}");
            assert!((a.1 - b.1).abs() > 0.2, "{shapes:?}");
        }
    }
}

#[test]
fn each_rotates_with_host() {
    let mut s = scene();
    let (clover, hearts) = (index(&s, "clover"), index(&s, "hearts"));
    let mut before = None;
    for t in [0.0f32, 10.0, 25.0] {
        s.clock.time = t;
        s.update();
        let mut seen = BTreeSet::new();
        for host in [clover, hearts] {
            // Over the most crowded sector (the pyramid on hearts), 3 R
            // out, looking at the planet's center.
            let local = if host == hearts {
                s.landmark.as_ref().unwrap().dir
            } else {
                let layers = s.props.as_ref().unwrap();
                let sectors = layers.layers.iter().flat_map(|l| &l.sectors);
                sectors.max_by_key(|sec| sec.props).unwrap().normal
            };
            let tr = s.bodies[host].transform;
            let out = tr.to_world_dir(local).normalized();
            s.ship.position = tr.position + out * (3.0 * s.radius_of(host));
            s.ship.forward = -out;
            s.ship.up = out.cross(Vector3::new(1.0, 0.0, 0.0)).normalized();
            s.ship.renormalize();
            let matrix = tr.matrix();
            let l = s.landmark.as_ref().unwrap();
            for d in s.effect_draws(W, H, 1) {
                let EffectShader::Prop(shader) = d.shader else {
                    continue;
                };
                let kind = if std::ptr::eq(d.mesh.as_ptr(), l.mesh.as_ptr())
                    || std::ptr::eq(d.mesh.as_ptr(), l.edges.as_ptr())
                {
                    Some(Kind::Pyramid)
                } else if std::ptr::eq(d.mesh.as_ptr(), l.disc.as_ptr()) {
                    // The disc turns on its own over the pyramid's top,
                    // which moves with the planet.
                    let m = d.model.model_matrix;
                    let origin = Vector3::new(m.m12, m.m13, m.m14);
                    let top = tr.position + tr.to_world_dir(l.disc_center) * tr.scale;
                    assert!((origin - top).length() < 1e-3, "t {t}");
                    None
                } else {
                    [
                        PropPart::TreeTrunk,
                        PropPart::TreeCanopy,
                        PropPart::Rock,
                        PropPart::BuildingBody,
                        PropPart::BuildingEdge,
                    ]
                    .into_iter()
                    .find(|p| p.color() == shader.color)
                    .map(kind_of)
                };
                if let Some(kind) = kind
                    && d.model.model_matrix == matrix
                {
                    seen.insert(kind);
                }
            }
        }
        assert_eq!(seen.len(), 4, "t {t}: {seen:?}");
        let yaws = (
            s.bodies[clover].transform.yaw,
            s.bodies[hearts].transform.yaw,
        );
        assert_ne!(before, Some(yaws), "the planets turn");
        before = Some(yaws);
    }
}

#[test]
fn crown_and_cards_not_counted() {
    let s = scene();
    for a in &s.attachments {
        let host = s.body_index(a.host).unwrap();
        match a.kind {
            // Ornaments of bodies without a surface: the crown on the star
            // (no orbit), the cards around House of Cards (a sphere).
            AttachmentKind::RoyalCrown | AttachmentKind::CardRing => {
                assert!(s.relief_of(host).is_none(), "{:?}", a.kind);
            }
            // Surface models: on a planet with its own ground.
            AttachmentKind::Props
            | AttachmentKind::City
            | AttachmentKind::Landmark
            | AttachmentKind::Crystals => {
                assert!(s.bodies[host].orbit.is_some() && s.relief_of(host).is_some());
            }
            // Water and the ships flying over Hearts (C65) are not
            // models on the ground.
            AttachmentKind::Waterfalls | AttachmentKind::Traffic => {}
        }
    }
    assert!(s.bodies[0].orbit.is_none());
    let crown = s
        .attachments
        .iter()
        .find(|a| a.kind == AttachmentKind::RoyalCrown)
        .unwrap();
    assert_eq!(s.body_index(crown.host), Some(0));
    // The counted kinds come from the surface models only.
    let kinds: BTreeSet<Kind> = placed(&s).iter().map(|p| p.0).collect();
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        [Kind::Tree, Kind::Rock, Kind::Building, Kind::Pyramid]
    );
}

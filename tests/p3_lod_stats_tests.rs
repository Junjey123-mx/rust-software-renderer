//! Project 3 per-planet workload: the level of detail and triangles of each
//! body in the frame counters, one HUD line per planet and the CSV
//! `lod_levels` column, all from the same counters.

use raylib::math::Vector3;
use rust_software_renderer::benchmark::solar::lod_levels;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::hud::planet_lines;
use rust_software_renderer::solar::instance::projected_radius_px;
use rust_software_renderer::solar::planet::geometry::lod_for;
use rust_software_renderer::solar::planet::{HeightField, TestField};
use rust_software_renderer::solar::system::SystemSpec;
use rust_software_renderer::solar::{SolarMeshes, SolarScene};

const W: u32 = 640;
const H: u32 = 480;

/// The project 3 system with a test relief on body 1, the ship `d` radii
/// in front of it.
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
    s.set_relief(
        1,
        HeightField::Test(TestField {
            amplitude: 0.04,
            frequency: 6.0,
            sea_level: None,
        }),
    )
    .unwrap();
    let p = s.bodies[1].transform.position;
    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(1));
    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
    s
}

fn render(s: &SolarScene) {
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
}

fn names(s: &SolarScene) -> Vec<&'static str> {
    s.bodies.iter().map(|b| b.id.0).collect()
}

#[test]
fn per_body_lod_matches_selection() {
    for d in [1.5, 4.0, 30.0] {
        let s = scene(d);
        render(&s);
        let stats = s.last_stats();
        let px = projected_radius_px(
            &s.projection(),
            s.bodies[1].transform.position,
            s.radius_of(1),
            H,
        );
        let relief = stats.per_body[1];
        assert!(relief.visible);
        assert_eq!(relief.lod, Some(lod_for(px)), "d {d}");
        let level = relief.lod.unwrap() as u32;
        assert_eq!(relief.triangles, 20 * 4usize.pow(level));
        // Sphere bodies carry no level (the clover planet has its own
        // relief since C49).
        for b in stats
            .per_body
            .iter()
            .filter(|b| s.relief_of(b.index).is_none())
        {
            assert_eq!(b.lod, None);
            assert_eq!(b.triangles, if b.visible { 192 } else { 0 });
        }
    }
}

#[test]
fn csv_lod_column_filled() {
    let s = scene(4.0);
    render(&s);
    let level = s.last_stats().per_body[1].lod.unwrap();
    let column = lod_levels(&s);
    let n = names(&s);
    let expected: Vec<String> = n
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let stats = s.last_stats();
            let own = stats
                .per_body
                .iter()
                .find(|b| b.index == i)
                .and_then(|b| b.lod);
            match (i, own) {
                (1, _) => format!("{name}:L{level}"),
                // The clover planet's own relief (C49).
                (_, Some(l)) if s.relief_of(i).is_some() => format!("{name}:L{l}"),
                _ => format!("{name}:-"),
            }
        })
        .collect();
    assert_eq!(column, expected.join(";"));
}

#[test]
fn hud_line_per_planet() {
    let s = scene(4.0);
    render(&s);
    let stats = s.last_stats();
    let lines = planet_lines(&stats, &names(&s));
    assert_eq!(lines.len(), s.bodies.len());
    let relief = stats.per_body[1];
    assert_eq!(
        lines[1],
        format!(
            "{} L{} {}T",
            names(&s)[1].to_uppercase(),
            relief.lod.unwrap(),
            relief.triangles
        )
    );
    // The HUD and the CSV read the same counters.
    assert!(lod_levels(&s).contains(&format!("L{}", relief.lod.unwrap())));
    for (line, b) in lines.iter().zip(stats.per_body.iter()) {
        if b.visible && b.lod.is_none() {
            assert!(line.ends_with(" SPHERE 192T"), "{line}");
        }
    }
}

#[test]
fn culled_planet_has_no_work() {
    // The ship looking away: the relief planet is culled.
    let mut s = scene(4.0);
    s.ship.forward = Vector3::new(0.0, 0.0, -1.0);
    render(&s);
    let stats = s.last_stats();
    let relief = stats.per_body[1];
    assert!(!relief.visible);
    assert_eq!((relief.triangles, relief.lod), (0, None));
    let lines = planet_lines(&stats, &names(&s));
    assert!(lines[1].ends_with(" CULLED"), "{}", lines[1]);
    drop(stats);
    assert!(lod_levels(&s).contains(&format!("{}:-", names(&s)[1])));
}

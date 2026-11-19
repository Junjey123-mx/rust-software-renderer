//! The royal crown of The King's Crown in the solar scene.

use raylib::math::Vector3;
use rust_software_renderer::geometry::Mesh;
use rust_software_renderer::solar::royal_crown::{CrownSpec, MAIN_POINTS, along, outward};
use rust_software_renderer::solar::shapes::{End, loft, octahedron};
use std::collections::HashMap;

/// Every directed edge has exactly one reverse twin: a closed surface whose
/// triangles all wind the same way.
fn is_closed(mesh: &Mesh) -> bool {
    let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
    for t in mesh.indices.chunks(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a, b)).or_default() += 1;
        }
    }
    edges
        .iter()
        .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
}

fn triangles(mesh: &Mesh) -> impl Iterator<Item = [Vector3; 3]> + '_ {
    mesh.indices.chunks(3).map(|t| {
        [
            mesh.vertices[t[0]],
            mesh.vertices[t[1]],
            mesh.vertices[t[2]],
        ]
    })
}

fn normal(t: &[Vector3; 3]) -> Vector3 {
    (t[1] - t[0]).cross(t[2] - t[0])
}

fn centroid(t: &[Vector3; 3]) -> Vector3 {
    (t[0] + t[1] + t[2]) / 3.0
}

/// Finite positions, no degenerate triangle.
fn is_sound(mesh: &Mesh) -> bool {
    mesh.vertices
        .iter()
        .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
        && triangles(mesh).all(|t| normal(&t).length() > 1e-7)
}

/// Every triangle of a convex piece faces away from its vertex mean.
fn faces_out_of(mesh: &Mesh) -> bool {
    let mean =
        mesh.vertices.iter().fold(Vector3::zero(), |a, &p| a + p) / mesh.vertices.len() as f32;
    triangles(mesh).all(|t| normal(&t).dot(centroid(&t) - mean) > 0.0)
}

#[test]
fn lofts_and_octahedra_are_closed_outward_solids() {
    let square = |y: f32, s: f32| {
        vec![
            Vector3::new(s, y, 0.0),
            Vector3::new(0.0, y, s),
            Vector3::new(-s, y, 0.0),
            Vector3::new(0.0, y, -s),
        ]
    };
    let blade = loft(
        &[square(0.0, 1.0), square(0.5, 1.2)],
        End::Cap,
        End::Apex(Vector3::new(0.0, 2.0, 0.0)),
        Vector3::new(0.0, 0.4, 0.0),
    );
    let gem = octahedron(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(0.3, 0.0, 0.0),
        Vector3::new(0.0, 0.5, 0.0),
        Vector3::new(0.0, 0.0, 0.2),
    );
    for mesh in [&blade, &gem] {
        assert!(is_closed(mesh) && is_sound(mesh) && faces_out_of(mesh));
    }
    assert_eq!(gem.triangle_count(), 8);
    assert_eq!(blade.triangle_count(), 8 + 2 + 4);
    // An open end leaves its ring's edges unmatched.
    let tube = loft(
        &[square(0.0, 1.0), square(1.0, 1.0)],
        End::Open,
        End::Cap,
        Vector3::new(0.0, 0.5, 0.0),
    );
    assert!(!is_closed(&tube) && faces_out_of(&tube));
}

#[test]
fn the_royal_crown_has_five_to_seven_main_points_of_several_heights() {
    let spec = CrownSpec::royal();
    let main: Vec<_> = spec.main_spikes().collect();
    assert_eq!(main.len(), MAIN_POINTS);
    assert!((5..=7).contains(&main.len()));
    let mut heights: Vec<i32> = main.iter().map(|s| (s.height * 1000.0) as i32).collect();
    heights.sort();
    heights.dedup();
    assert!(heights.len() >= 3, "{heights:?}");
    // The front point is the tallest and the crown is mirror-symmetric.
    let tallest = main
        .iter()
        .max_by(|a, b| a.height.total_cmp(&b.height))
        .unwrap();
    assert_eq!(tallest.azimuth, 0.0);
    for s in &spec.spikes {
        let mirror = spec
            .spikes
            .iter()
            .find(|m| (outward(m.azimuth) - outward(-s.azimuth)).length() < 1e-4)
            .expect("every point has a mirror twin");
        assert_eq!((mirror.height, mirror.width), (s.height, s.width));
    }
    assert_eq!(spec.gems.len(), MAIN_POINTS);
    assert!(
        spec.spikes.len() > main.len(),
        "small points between the main ones"
    );
}

#[test]
fn every_piece_is_a_sound_closed_mesh_facing_outwards() {
    let spec = CrownSpec::royal();
    let band = spec.band.mesh();
    assert!(is_closed(&band) && is_sound(&band));
    assert_eq!(band.triangle_count(), spec.band.segments * 8);
    // The band is a ring: each face looks away from the middle of its own
    // cross-section.
    let middle = spec.band.latitude.sin();
    for t in triangles(&band) {
        let c = centroid(&t);
        let azimuth = c.x.atan2(c.z);
        let section =
            outward(azimuth) * spec.band.radius_at(middle) + Vector3::new(0.0, middle, 0.0);
        assert!(normal(&t).dot(c - section) > 0.0, "band face at {c:?}");
    }
    for s in &spec.spikes {
        let blade = s.mesh(&spec.band);
        assert!(is_closed(&blade) && is_sound(&blade) && faces_out_of(&blade));
        if let Some(f) = s.finial_mesh(&spec.band) {
            assert!(is_closed(&f) && is_sound(&f) && faces_out_of(&f));
        }
    }
    for g in &spec.gems {
        let gem = g.mesh(&spec.band);
        assert!(is_closed(&gem) && is_sound(&gem) && faces_out_of(&gem));
    }
}

#[test]
fn the_band_sits_on_the_upper_star_and_the_points_rise_outwards() {
    let spec = CrownSpec::royal();
    let band = spec.band.mesh();
    let (bottom, top) = (spec.band.bottom(), spec.band.top());
    for p in &band.vertices {
        if (p.y - bottom).abs() < 1e-5 {
            assert!(
                p.length() < 0.97,
                "lower edge sunk into the sphere: {}",
                p.length()
            );
        } else {
            assert!(
                p.length() > 1.0,
                "upper edge outside the sphere: {}",
                p.length()
            );
        }
    }
    assert!(bottom > 0.5 && top < 0.95, "on the northern cap");
    for s in &spec.spikes {
        let (axis, tip) = (s.axis(), s.tip(&spec.band));
        assert!(
            axis.y > 0.9 && axis.dot(outward(s.azimuth)) > 0.0,
            "up and slightly out"
        );
        assert!(axis.dot(along(s.azimuth)).abs() < 1e-6, "no sideways tilt");
        assert!(tip.y > top && tip.length() > 1.1, "{tip:?}");
    }
}

#[test]
fn open_gaps_separate_the_points_so_the_star_shows_between_them() {
    let spec = CrownSpec::royal();
    let mut points = spec.spikes.clone();
    points.sort_by(|a, b| a.azimuth.total_cmp(&b.azimuth));
    let rim = spec.band.radius_at(spec.band.top());
    for (i, a) in points.iter().enumerate() {
        let b = &points[(i + 1) % points.len()];
        let gap = (b.azimuth - a.azimuth).rem_euclid(std::f32::consts::TAU) * rim;
        let clear =
            gap - (a.width + b.width) / 2.0 * rust_software_renderer::solar::royal_crown::BULGE;
        assert!(
            clear > 0.03,
            "points {i} and {} touch: {clear}",
            (i + 1) % points.len()
        );
    }
}

use rust_software_renderer::solar::SPHERE_RADIUS;
use rust_software_renderer::solar::royal_crown::RoyalCrown;

#[test]
fn the_crown_mesh_is_closed_per_material_and_reproducible() {
    let spec = CrownSpec::royal();
    for mesh in [spec.gold_mesh(), spec.jewel_mesh()] {
        assert!(is_closed(&mesh) && is_sound(&mesh));
    }
    let (a, b) = (RoyalCrown::royal(), RoyalCrown::royal());
    let positions = |v: &[rust_software_renderer::geometry::Vertex]| {
        v.iter().map(|c| c.position).collect::<Vec<_>>()
    };
    assert_eq!(positions(&a.gold), positions(&b.gold));
    assert_eq!(positions(&a.jewels), positions(&b.jewels));
    assert_eq!(a.gold.len() % 3, 0);
    assert_eq!(a.jewels.len() % 3, 0);
    // A clone shares the vertex lists instead of copying them.
    let c = a.clone();
    assert!(std::sync::Arc::ptr_eq(&a.gold, &c.gold));
}

#[test]
fn the_main_points_are_spread_evenly_around_the_rim() {
    let spec = CrownSpec::royal();
    let tips: Vec<Vector3> = spec.main_spikes().map(|s| s.tip(&spec.band)).collect();
    let step = std::f32::consts::TAU / MAIN_POINTS as f32;
    for (i, s) in spec.main_spikes().enumerate() {
        assert!((s.azimuth - step * i as f32).abs() < 1e-5);
        let t = tips[i];
        let seen = t.x.atan2(t.z).rem_euclid(std::f32::consts::TAU);
        assert!((seen - s.azimuth.rem_euclid(std::f32::consts::TAU)).abs() < 1e-4);
    }
    for (i, a) in tips.iter().enumerate() {
        let b = tips[(i + 1) % tips.len()];
        assert!((*a - b).length() > 0.3, "neighbouring tips apart");
    }
}

#[test]
fn the_built_crown_is_scaled_to_the_course_sphere_and_stays_light() {
    let crown = RoyalCrown::royal();
    let gold = crown.spec.gold_mesh();
    let expected: Vec<Vector3> = gold
        .indices
        .iter()
        .map(|&i| gold.vertices[i] * SPHERE_RADIUS)
        .collect();
    let built: Vec<Vector3> = crown.gold.iter().map(|v| v.position).collect();
    assert_eq!(built, expected);
    let reach = crown.extent * SPHERE_RADIUS;
    let farthest = crown
        .gold
        .iter()
        .chain(crown.jewels.iter())
        .map(|v| v.position.length())
        .fold(0.0, f32::max);
    assert!((farthest - reach).abs() < 1e-5, "{farthest} vs {reach}");
    let tallest = crown
        .spec
        .main_spikes()
        .map(|s| s.tip(&crown.spec.band).length())
        .fold(0.0, f32::max);
    assert!(
        crown.extent > tallest && crown.extent < 1.8,
        "{}",
        crown.extent
    );
    let n = crown.triangle_count();
    assert!(n > 300 && n < 800, "{n} triangles");
}

// ---------------------------------------------------------------------------
// The crown on the star in the solar scene
// ---------------------------------------------------------------------------

use rust_software_renderer::celestial::Body;
use rust_software_renderer::pipeline::{
    OrbitCamera, OrbitView, ShadedOptions, Transform, ViewPreset, Viewport,
};
use rust_software_renderer::rasterization::TriangleMode;
use rust_software_renderer::rendering::Framebuffer;
use rust_software_renderer::solar::effects::EffectShader;
use rust_software_renderer::solar::plasma_loops::{CROWN_ARCS, PlasmaLoops};
use rust_software_renderer::solar::{BodyInstance, CameraMode, SolarMeshes, SolarScene};

const W: u32 = 320;
const H: u32 = 240;

/// The crown's star alone at the origin under the orthographic front
/// camera, framed so the crown fits above it.
fn star_scene(spin: f32, time: f32) -> SolarScene {
    let camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Front);
    let view = OrbitView::for_world(camera, 2.4, Viewport::new(W, H, 0.1)).unwrap();
    let mut s = SolarScene::new(
        SolarMeshes::load().unwrap(),
        view,
        Vector3::new(0.0, 0.0, -1.0),
    );
    s.camera_mode = CameraMode::Orbit;
    s.bodies = vec![BodyInstance::new(
        Body::KingsCrown,
        Transform::new(Vector3::zero(), 1.3),
        spin,
    )];
    s.plasma = PlasmaLoops::new(&[]);
    s.clock.time = time;
    s.update();
    s
}

fn frame(s: &SolarScene) -> Framebuffer {
    let mut fb = Framebuffer::new(W, H);
    s.render_frame(
        &mut fb,
        TriangleMode::FilledScanline,
        ShadedOptions::default(),
    )
    .unwrap();
    fb
}

fn without_crown(mut s: SolarScene) -> SolarScene {
    s.crown = None;
    s
}

/// Pixels whose depth the crown changed.
fn crown_pixels(s: &SolarScene) -> Vec<(i32, i32)> {
    let (with, without) = (frame(s), frame(&without_crown(s.clone())));
    let mut covered = Vec::new();
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            if with.get_depth(x, y) != without.get_depth(x, y) {
                covered.push((x, y));
            }
        }
    }
    covered
}

#[test]
fn the_crown_rises_above_the_star_and_also_covers_its_upper_disk() {
    let s = star_scene(0.05, 4.0);
    let covered = crown_pixels(&s);
    let radius_px = s.radius_of(0) * s.view.scale();
    let (cx, cy) = (W as f32 / 2.0, H as f32 / 2.0);
    let above = covered
        .iter()
        .filter(|&&(_, y)| (y as f32 + 0.5) < cy - radius_px)
        .count();
    let on_disk = covered
        .iter()
        .filter(|&&(x, y)| {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            (dx * dx + dy * dy).sqrt() < radius_px * 0.95
        })
        .count();
    assert!(above > 150, "{above} pixels above the star");
    assert!(on_disk > 150, "{on_disk} pixels in front of the star");
    assert!(
        covered.iter().all(|&(_, y)| (y as f32) < cy),
        "the crown sits on the northern half"
    );
}

#[test]
fn the_star_hides_the_back_of_the_crown() {
    let s = star_scene(0.05, 4.0);
    let options = ShadedOptions::default();
    let shaded = |fb: &mut Framebuffer| -> usize {
        s.render_effects_without_clear(fb, TriangleMode::FilledScanline, options)
            .unwrap()
            .iter()
            .map(|d| d.fragments_shaded)
            .sum()
    };
    let mut alone = Framebuffer::new(W, H);
    alone.clear();
    let free = shaded(&mut alone);
    let mut behind = Framebuffer::new(W, H);
    behind.clear();
    s.render_objects_without_clear(&mut behind, TriangleMode::FilledScanline, options)
        .unwrap();
    let occluded = shaded(&mut behind);
    assert!(occluded > 0 && occluded < free, "{occluded} of {free}");
}

#[test]
fn the_crown_rides_on_the_star_transform_with_an_optional_own_spin() {
    let mut s = star_scene(0.05, 7.0);
    s.bodies[0].transform.position = Vector3::new(0.3, -0.2, 0.1);
    s.bodies[0].transform.scale = 0.9;
    let star = s.bodies[0].transform;
    assert_eq!(s.crown_transform(0), star);
    s.crown.as_mut().unwrap().spin = 0.1;
    let own = s.crown_transform(0);
    assert!((own.yaw - (star.yaw + 0.1 * 7.0)).abs() < 1e-6);
    assert_eq!(
        (own.position, own.scale, own.pitch),
        (star.position, star.scale, star.pitch)
    );
    // On screen: the covered pixels move with a turning star and stay put
    // on a still one.
    let still = |t| crown_pixels(&star_scene(0.0, t));
    assert_eq!(still(2.0), still(9.0));
    assert_ne!(crown_pixels(&star_scene(0.05, 9.0)), still(9.0));
}

#[test]
fn without_the_star_or_without_a_crown_nothing_is_attached() {
    let s = star_scene(0.05, 4.0);
    assert_eq!(s.crown_host(), Some(0));
    assert_eq!(crown_draws(&s), 2, "gold and jewels");
    let bare = without_crown(s.clone());
    assert_eq!(bare.crown_host(), None);
    assert_eq!(crown_draws(&bare), 0);
    let mut other = s.clone();
    other.bodies[0] = BodyInstance::new(
        Body::HouseOfCards,
        Transform::new(Vector3::zero(), 1.3),
        0.05,
    );
    assert_eq!(crown_draws(&other), 0, "the gas giant wears no crown");
}

fn crown_draws(s: &SolarScene) -> usize {
    s.effect_draws(W, H, 1)
        .iter()
        .filter(|d| matches!(d.shader, EffectShader::Crown(_)))
        .count()
}

#[test]
fn the_crown_keeps_clear_of_the_plasma_loops_and_both_are_drawn() {
    let crown = RoyalCrown::royal();
    let corners: Vec<Vector3> = crown
        .gold
        .iter()
        .chain(crown.jewels.iter())
        .map(|v| v.position)
        .collect();
    for (i, arc) in CROWN_ARCS.iter().enumerate() {
        for k in 0..=60 {
            let t = k as f32 / 60.0;
            let axis = arc.point(t, SPHERE_RADIUS);
            let tube = arc.width_at(t) * SPHERE_RADIUS;
            let nearest = corners
                .iter()
                .map(|&c| (c - axis).length())
                .fold(f32::INFINITY, f32::min);
            assert!(nearest > tube + 0.05, "arc {i} at t {t}: {nearest}");
        }
    }
    // With both attached, the loops still change the frame.
    let mut s = star_scene(0.05, 6.0);
    s.plasma = PlasmaLoops::crown();
    let mut bare = s.clone();
    bare.plasma = PlasmaLoops::new(&[]);
    let (with, without) = (frame(&s), frame(&bare));
    let changed = (0..H as i32)
        .flat_map(|y| (0..W as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| with.get_depth(x, y) != without.get_depth(x, y))
        .count();
    assert!(changed > 200, "{changed}");
}

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

use rust_software_renderer::solar::royal_crown::CrownPart;

fn luminance(c: Vector3) -> f32 {
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z
}

fn is_gold(c: Vector3) -> bool {
    c.x > 0.3 && c.y > 0.45 * c.x && c.z < 0.8 * c.y
}

fn is_red(c: Vector3) -> bool {
    c.x > 0.2 && c.x > 2.2 * c.y && c.x > 1.5 * c.z
}

#[test]
fn the_crown_reads_as_gold_metal_with_red_jewels_against_the_sky() {
    let s = star_scene(0.05, 4.0);
    let fb = frame(&s);
    let radius_px = s.radius_of(0) * s.view.scale();
    let top = H as f32 / 2.0 - radius_px;
    let sky: Vec<Vector3> = crown_pixels(&s)
        .into_iter()
        .filter(|&(_, y)| (y as f32 + 0.5) < top)
        .map(|(x, y)| fb.get_pixel(x, y).unwrap())
        .collect();
    let gold = sky.iter().filter(|&&c| is_gold(c)).count();
    let red = sky.iter().filter(|&&c| is_red(c)).count();
    assert!(gold > sky.len() / 2, "{gold} gold of {}", sky.len());
    assert!(red > 15 && red < gold, "{red} red, {gold} gold");
}

#[test]
fn the_band_wears_a_corinto_enamel_stripe_between_gold_edges() {
    let crown = RoyalCrown::royal();
    let band = crown.spec.band;
    let shader = crown.shader(CrownPart::Gold, Transform::default(), 0.0);
    let eye = Vector3::new(0.0, 0.0, 1.0);
    let normal = outward(0.0);
    let stripe = shader.shade(band.outer_face(0.0) * SPHERE_RADIUS, normal, eye);
    let y = band.top() - 0.01;
    let edge = (outward(0.0) * (band.radius_at(y) + band.thickness / 2.0)
        + Vector3::new(0.0, y, 0.0))
        * SPHERE_RADIUS;
    let edge = shader.shade(edge, normal, eye);
    assert!(is_red(stripe) && stripe.x < 0.9, "corinto: {stripe:?}");
    assert!(is_gold(edge), "gold: {edge:?}");
    // The inner face of a point mirrors the star: brighter than its back.
    let point = crown.spec.main_spikes().next().unwrap();
    let mid = (point.base(&band) + point.tip(&band)) / 2.0 * SPHERE_RADIUS;
    let inward = shader.shade(mid, -outward(0.0), -eye);
    let outer = shader.shade(mid, outward(0.0), eye);
    assert!(
        luminance(inward) > luminance(outer),
        "{inward:?} vs {outer:?}"
    );
}

#[test]
fn gems_and_finials_are_different_stones_that_shimmer_while_gold_holds_still() {
    let crown = RoyalCrown::royal();
    let band = crown.spec.band;
    let eye = Vector3::new(0.0, 0.0, 1.0);
    let at = |part, t| crown.shader(part, Transform::default(), t);
    let gem = crown.spec.gems[0].center(&band) * SPHERE_RADIUS + outward(0.0) * 0.01;
    let point = crown.spec.main_spikes().next().unwrap();
    let finial = point.tip(&band) * SPHERE_RADIUS;
    let n = outward(0.0);
    let (g0, g1) = (
        at(CrownPart::Jewels, 0.0).shade(gem, n, eye),
        at(CrownPart::Jewels, 1.0).shade(gem, n, eye),
    );
    let f0 = at(CrownPart::Jewels, 0.0).shade(finial, n, eye);
    assert!(is_red(g0) && is_red(f0), "{g0:?} {f0:?}");
    assert_ne!(g0, f0, "ruby and garnet");
    assert_ne!(g0, g1, "the gems shimmer");
    let gold = |t| at(CrownPart::Gold, t).shade(finial, n, eye);
    assert_eq!(gold(0.0), gold(1.0));
}

#[test]
fn the_crown_stays_legible_from_several_viewpoints() {
    for preset in [
        ViewPreset::Front,
        ViewPreset::Side,
        ViewPreset::Top,
        ViewPreset::Oblique,
    ] {
        let mut s = star_scene(0.05, 4.0);
        s.view.camera = OrbitCamera::new(Vector3::zero(), preset);
        let fb = frame(&s);
        let lum: Vec<f32> = crown_pixels(&s)
            .into_iter()
            .map(|(x, y)| luminance(fb.get_pixel(x, y).unwrap()))
            .collect();
        let n = lum.len() as f32;
        let mean = lum.iter().sum::<f32>() / n;
        let spread = (lum.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / n).sqrt();
        assert!(lum.len() > 300, "{preset:?}: {} pixels", lum.len());
        assert!(mean > 0.25 && mean < 1.5, "{preset:?}: mean {mean}");
        assert!(spread > 0.05, "{preset:?}: facets visible ({spread})");
    }
}

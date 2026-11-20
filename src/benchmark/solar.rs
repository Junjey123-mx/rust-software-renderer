//! Camino al Proyecto 3: headless benchmark of reproducible solar scenes.
//!
//! Every scenario fixes the simulated time, the star seed, the camera and
//! the spacecraft, then times whole CPU frames (`SolarScene::render_frame`:
//! clear, stars, bodies and spacecraft). Window presentation
//! (`to_rgba_bytes` and the texture upload) is not included.

use crate::pipeline::{OrbitCamera, OrbitView, ShadedOptions, ViewPreset, Viewport};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use crate::solar::camera::CameraRig;
use crate::solar::flight::FlightMode;
use crate::solar::planet::{HeightField, TestField};
use crate::solar::skybox::StarField;
use crate::solar::system::PlanetId;
use crate::solar::{CameraMode, FrameStats, SolarMeshes, SolarScene};
use raylib::math::Vector3;
use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub struct SolarBenchConfig {
    pub width: u32,
    pub height: u32,
    pub warmup: usize,
    pub rounds: usize,
    /// Shader threads; 0 means all available cores.
    pub threads: usize,
    /// Simulated time of every scenario.
    pub time: f32,
}

impl Default for SolarBenchConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            warmup: 1,
            rounds: 7,
            threads: 0,
            time: 6.0,
        }
    }
}

/// The scenarios A-J of the pre-project plan.
pub const SCENARIOS: [(&str, &str); 10] = [
    ("A", "spacecraft alone"),
    ("B", "one planet"),
    ("C", "distant system with three bodies"),
    ("D", "full system with the spacecraft"),
    ("E", "close approach to Crimson Entrance"),
    ("F", "close approach to House of Cards"),
    ("G", "camera facing empty space"),
    ("H", "full system with the star field"),
    ("I", "full system without the sky (stars and nebula)"),
    ("J", "lowest flight over Crimson Entrance"),
];

/// Project 3 culling cases, outside A-J (whose rows, CSV and frozen
/// fingerprints stay as they are); `scenario_scene` builds them too.
pub const CULLING_SCENARIOS: [(&str, &str); 2] = [
    ("G2", "planet cut by the screen edge"),
    ("G3", "camera turned 90 degrees"),
];

/// Project 3 relief cases, scaled system only: Crimson Entrance with a test
/// relief far (level 2), mid-range (level 4) and grazing (level 4 at
/// 800x600, 5 on taller frames), each with the automatic level and then
/// with level 5 forced (A/B side by side).
pub const LOD_SCENARIOS: [(&str, &str); 6] = [
    ("L1", "relief far with automatic LOD"),
    ("L1F", "relief far with level 5 forced"),
    ("L2", "relief mid-range with automatic LOD"),
    ("L2F", "relief mid-range with level 5 forced"),
    ("L3", "relief grazing with automatic LOD"),
    ("L3F", "relief grazing with level 5 forced"),
];

/// Project 3 clover cases, scaled system only: the clover planet up close
/// (relief, sea, trees and rocks), from afar, and facing a mountain with
/// its waterfall.
pub const CLOVER_SCENARIOS: [(&str, &str); 3] = [
    ("K", "close flight over the clover planet"),
    ("K2", "clover planet from afar"),
    ("K3", "clover mountain and waterfall up close"),
];

/// Project 3 Hearts cases, scaled system only: the pyramid and the city up
/// close, and the planet from afar.
pub const HEARTS_SCENARIOS: [(&str, &str); 2] = [
    ("M", "close flight over the hearts pyramid and city"),
    ("M2", "hearts planet from afar"),
];

/// Test relief of the `L` scenarios (benchmark only).
pub const BENCH_RELIEF: HeightField = HeightField::Test(TestField {
    amplitude: 0.04,
    frequency: 6.0,
    sea_level: Some(0.0),
});

#[derive(Debug, Clone, PartialEq)]
pub struct SolarBenchRow {
    pub id: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub threads: usize,
    pub rounds: usize,
    pub bodies: usize,
    pub fragments_shaded: usize,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    /// Slowest timed round (with `min_ms`, the spread of the rounds).
    pub max_ms: f64,
    /// Peak resident memory of the process after the scenario, kilobytes
    /// (`VmHWM` of `/proc/self/status`; `None` where it is not available).
    pub peak_rss_kb: Option<u64>,
    /// Counters of the last timed frame (identical in every round).
    pub stats: FrameStats,
    /// Level of detail of each body, `name:level` joined by `;` (`-` while
    /// every body draws the one sphere mesh).
    pub lod_levels: String,
    /// Star-field seed of the scenario.
    pub seed: u32,
    /// Commit measured: `P3_COMMIT` from the environment (letters and
    /// digits only), empty when unset.
    pub commit: String,
}

/// Version of the solar CSV layout: 2 appends the frame counters, the level
/// of detail, the seed and the commit after the version 1 columns.
pub const SOLAR_CSV_VERSION: u32 = 2;

/// Columns of the solar CSV, in order.
pub const SOLAR_CSV_HEADER: &str = "scenario,name,width,height,threads,rounds,bodies,fragments_shaded,median_ms,p95_ms,min_ms,max_ms,peak_rss_kb,csv_version,objects_tested,objects_culled,draws,triangles_submitted,triangles_culled,lod_levels,seed,commit";

/// `P3_COMMIT` reduced to letters and digits (it lands in a CSV field).
fn commit_from_env() -> String {
    std::env::var("P3_COMMIT")
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// Level of detail of every body in the last frame of `scene`, for the CSV:
/// `name:L3` for a relief level, `name:-` for the course sphere or a body
/// that was not drawn.
pub fn lod_levels(scene: &SolarScene) -> String {
    let stats = scene.last_stats();
    scene
        .bodies
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let lod = stats
                .per_body
                .iter()
                .find(|s| s.index == i)
                .and_then(|s| s.lod);
            match lod {
                Some(l) => format!("{}:L{l}", b.id.0),
                None => format!("{}:-", b.id.0),
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Peak resident set size of this process in kilobytes, read from
/// `/proc/self/status` (Linux); `None` elsewhere or when unreadable.
pub fn peak_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
}

/// Parks the spacecraft in local flight `altitude` above body `index`,
/// nose along the surface, pitched down by `pitch_down` radians.
fn hover(scene: &mut SolarScene, index: usize, altitude: f32, pitch_down: f32) {
    let center = scene.bodies[index].transform.position;
    let up = (center - scene.center).normalized();
    scene.ship.position = center + up * (scene.radius_of(index) + altitude);
    scene.ship.forward = Vector3::new(0.0, 1.0, 0.0).cross(up).normalized();
    scene.ship.up = up;
    scene.ship.renormalize();
    scene.ship.pitch(-pitch_down);
    scene.flight = FlightMode::Space;
    scene
        .toggle_local()
        .expect("hover height is inside the local range");
}

/// Builds the scene of scenario `id` at the configured size and time, on
/// the pre-project system.
pub fn scenario_scene(id: &str, config: &SolarBenchConfig) -> Result<SolarScene, Box<dyn Error>> {
    scenario_scene_on(id, config, false)
}

/// `scenario_scene` on the Project 3 system at the project scale
/// (`SolarScene::project3_system`) when `scaled`, framed by its layout.
pub fn scenario_scene_on(
    id: &str,
    config: &SolarBenchConfig,
    scaled: bool,
) -> Result<SolarScene, Box<dyn Error>> {
    let camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    let viewport = Viewport::new(config.width, config.height, 0.1);
    let view = OrbitView::for_world(camera, crate::solar::viewer::WORLD_RADIUS, viewport)?;
    let (meshes, light) = (SolarMeshes::load()?, Vector3::new(0.0, 1.0, 0.0));
    let mut s = if scaled {
        let mut s = SolarScene::project3_system(meshes, view, light)?;
        if let Some(layout) = &s.layout {
            s.view = OrbitView::for_world(camera, layout.world_radius, viewport)?;
        }
        s
    } else {
        SolarScene::default_system(meshes, view, light)?
    };
    // Hover heights: world units on the pre-project system, planet radii on
    // the scaled one (where J flies at the planet's own clearance).
    let height = |s: &SolarScene, i: usize, h: f32| {
        if scaled { h * s.radius_of(i) } else { h }
    };
    s.camera_mode = CameraMode::Perspective;
    s.clock.time = config.time;
    s.update();
    match id {
        "A" => s.bodies.clear(),
        "B" => {
            s.bodies
                .retain(|b| b.id.0 == crate::celestial::Body::CrimsonEntrance.name());
            s.rig = CameraRig::Orbit;
            s.view.camera.target = s.bodies[0].transform.position;
            s.view.camera.zoom = 2.5;
        }
        "C" => s.rig = CameraRig::Orbit,
        "D" | "H" => {}
        "E" => {
            let h = height(&s, 1, 0.7);
            hover(&mut s, 1, h, 0.5)
        }
        "F" => {
            // House of Cards by id: the clover sits before it in Project 3.
            let house = s.body_index(PlanetId("house")).ok_or("no House of Cards")?;
            let h = height(&s, house, 0.7);
            hover(&mut s, house, h, 0.5)
        }
        "K" | "K2" | "K3" if scaled => {
            let clover = s.body_index(PlanetId("clover")).ok_or("no clover planet")?;
            if id == "K" {
                // Over the clover's land, nose down towards its mountains.
                // `hover` flies over the night side; turning the pose 180°
                // about the ship's right axis through the planet's center
                // brings it over the day side.
                let h = height(&s, clover, 0.45);
                hover(&mut s, clover, h, 0.55);
                let c = s.bodies[clover].transform.position;
                s.ship.position = c - (s.ship.position - c);
                s.ship.forward = -s.ship.forward;
                s.ship.up = -s.ship.up;
            } else if id == "K3" {
                // Over the sea below the most sunlit waterfall, facing it
                // and its mountain.
                let t = s.bodies[clover].transform;
                let world = |v: Vector3| t.position + t.to_world_dir(v) * t.scale;
                let sun = (s.center - t.position).normalized();
                let falls = &s.waterfalls.as_ref().ok_or("no waterfalls")?.falls;
                let fall = falls
                    .iter()
                    .max_by(|a, b| {
                        let lit = |f: &&crate::solar::attach::waterfalls::PlacedWaterfall| {
                            t.to_world_dir(f.normal).dot(sun)
                        };
                        lit(a).total_cmp(&lit(b))
                    })
                    .ok_or("no waterfalls")?;
                let (top, bottom) = (world(fall.fall.top), world(fall.fall.bottom));
                let out = (bottom - t.position).normalized();
                let away = bottom - top;
                let away = (away - out * away.dot(out)).normalized();
                let r = s.radius_of(clover);
                s.ship.position = bottom + away * (0.3 * r) + out * (0.12 * r);
                s.ship.forward = ((top + bottom) * 0.5 - s.ship.position).normalized();
                s.ship.up = out;
                s.ship.renormalize();
            } else {
                // 12 radii away, in front of it.
                let p = s.bodies[clover].transform.position;
                s.ship.position = p - Vector3::new(0.0, 0.0, 12.0 * s.radius_of(clover));
                s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
                s.ship.up = Vector3::new(0.0, 1.0, 0.0);
            }
        }
        "M" | "M2" if scaled => {
            let hearts = s.body_index(PlanetId("hearts")).ok_or("no hearts planet")?;
            let t = s.bodies[hearts].transform;
            let r = s.radius_of(hearts);
            if id == "M" {
                // Beside the pyramid, a third of a radius over its top,
                // looking at its foot across the city (the pyramid and
                // its disc rise above the ship).
                let l = s.landmark.as_ref().ok_or("no hearts landmark")?;
                let top = t.position + t.to_world_dir(l.top_local) * t.scale;
                let foot = t.position + t.to_world_dir(l.dir) * r;
                let up = t.to_world_dir(l.up).normalized();
                let side = up.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
                s.ship.position = top + up * (0.35 * r) + side * (0.9 * r);
                s.ship.forward = (foot - s.ship.position).normalized();
                s.ship.up = up;
                s.ship.renormalize();
            } else {
                // 12 radii away, in front of it.
                s.ship.position = t.position - Vector3::new(0.0, 0.0, 12.0 * r);
                s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
                s.ship.up = Vector3::new(0.0, 1.0, 0.0);
            }
        }
        "G" => s.ship.yaw(std::f32::consts::PI),
        // Crimson Entrance cut about in half by the left edge (found by a
        // sweep: the whole planet covers about 4000 pixels at 800x600, 2300
        // stay on screen, 66 of them on the first column).
        "G2" => s.ship.yaw(-0.38),
        "G3" => s.ship.yaw(std::f32::consts::FRAC_PI_2),
        "I" => {
            s.stars = StarField::generate(s.stars.seed, 0);
            s.nebula.intensity = 0.0;
        }
        "J" => {
            let clearance = s.flight_scale(1).clearance;
            hover(&mut s, 1, clearance, 0.8)
        }
        "L1" | "L1F" | "L2" | "L2F" | "L3" | "L3F" if scaled => {
            s.set_relief(1, BENCH_RELIEF)?;
            if id.ends_with('F') {
                s.lod_force = Some(5);
            }
            match &id[..2] {
                // In front of the planet, 15 and 2 radii from its center.
                "L1" | "L2" => {
                    let d = if id.starts_with("L1") { 15.0 } else { 2.0 };
                    let p = s.bodies[1].transform.position;
                    s.ship.position = p - Vector3::new(0.0, 0.0, d * s.radius_of(1));
                    s.ship.forward = Vector3::new(0.0, 0.0, 1.0);
                    s.ship.up = Vector3::new(0.0, 1.0, 0.0);
                }
                // J's grazing flight.
                _ => {
                    let clearance = s.flight_scale(1).clearance;
                    hover(&mut s, 1, clearance, 0.8)
                }
            }
        }
        other => return Err(format!("unknown solar scenario '{other}'").into()),
    }
    Ok(s)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

pub fn run_solar_benchmark(
    config: &SolarBenchConfig,
) -> Result<Vec<SolarBenchRow>, Box<dyn Error>> {
    run_solar_benchmark_on(config, false)
}

/// `run_solar_benchmark` on the Project 3 system when `scaled`
/// (`scenario_scene_on`).
pub fn run_solar_benchmark_on(
    config: &SolarBenchConfig,
    scaled: bool,
) -> Result<Vec<SolarBenchRow>, Box<dyn Error>> {
    if config.rounds == 0 {
        return Err("the solar benchmark needs at least one round".into());
    }
    let threads = match config.threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    };
    let options = ShadedOptions {
        threads,
        ..ShadedOptions::default()
    };
    let mut rows = Vec::new();
    let p3: Vec<(&str, &str)> = if scaled {
        LOD_SCENARIOS
            .iter()
            .chain(&CLOVER_SCENARIOS)
            .chain(&HEARTS_SCENARIOS)
            .copied()
            .collect()
    } else {
        Vec::new()
    };
    for &(id, name) in SCENARIOS.iter().chain(&p3) {
        let scene = scenario_scene_on(id, config, scaled)?;
        let mut fb = Framebuffer::new(config.width, config.height);
        let mut times = Vec::new();
        let mut fragments = 0;
        for round in 0..config.warmup + config.rounds {
            let start = Instant::now();
            fragments = scene.render_frame(&mut fb, TriangleMode::FilledScanline, options)?;
            let ms = start.elapsed().as_secs_f64() * 1e3;
            if round >= config.warmup {
                times.push(ms);
            }
        }
        times.sort_by(f64::total_cmp);
        let lod_levels = lod_levels(&scene);
        rows.push(SolarBenchRow {
            id,
            name,
            width: config.width,
            height: config.height,
            threads,
            rounds: config.rounds,
            bodies: scene.bodies.len(),
            fragments_shaded: fragments,
            median_ms: percentile(&times, 0.5),
            p95_ms: percentile(&times, 0.95),
            min_ms: times[0],
            max_ms: times[times.len() - 1],
            peak_rss_kb: peak_rss_kb(),
            stats: scene.last_stats().clone(),
            lod_levels,
            seed: scene.stars.seed,
            commit: commit_from_env(),
        });
    }
    Ok(rows)
}

pub fn write_solar_csv(rows: &[SolarBenchRow], path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(out, "{SOLAR_CSV_HEADER}")?;
    for r in rows {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{SOLAR_CSV_VERSION},{},{},{},{},{},{},{},{}",
            r.id,
            r.name,
            r.width,
            r.height,
            r.threads,
            r.rounds,
            r.bodies,
            r.fragments_shaded,
            r.median_ms,
            r.p95_ms,
            r.min_ms,
            r.max_ms,
            r.peak_rss_kb.map_or(String::new(), |kb| kb.to_string()),
            r.stats.objects_tested,
            r.stats.objects_culled,
            r.stats.draws,
            r.stats.triangles_submitted,
            r.stats.triangles_culled,
            r.lod_levels,
            r.seed,
            r.commit
        )?;
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_spread_brackets_the_median_and_memory_is_read() {
        let config = SolarBenchConfig {
            width: 64,
            height: 48,
            warmup: 0,
            rounds: 3,
            threads: 1,
            time: 6.0,
        };
        for r in run_solar_benchmark(&config).unwrap() {
            assert!(r.min_ms <= r.median_ms && r.median_ms <= r.max_ms, "{r:?}");
            if cfg!(target_os = "linux") {
                assert!(r.peak_rss_kb.is_some_and(|kb| kb > 0), "{r:?}");
            }
        }
    }

    #[test]
    fn csv_header_keeps_the_old_columns_first() {
        let dir = std::env::temp_dir().join(format!("solar_csv_{}", std::process::id()));
        let path = dir.join("solar.csv");
        let row = SolarBenchRow {
            id: "A",
            name: "spacecraft alone",
            width: 1,
            height: 1,
            threads: 1,
            rounds: 1,
            bodies: 0,
            fragments_shaded: 0,
            median_ms: 1.0,
            p95_ms: 1.0,
            min_ms: 1.0,
            max_ms: 1.0,
            peak_rss_kb: None,
            stats: FrameStats::default(),
            lod_levels: String::new(),
            seed: 42,
            commit: String::new(),
        };
        write_solar_csv(&[row], &path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(text.starts_with(
            "scenario,name,width,height,threads,rounds,bodies,fragments_shaded,median_ms,p95_ms,min_ms,max_ms,peak_rss_kb,csv_version,"
        ));
        assert!(text.lines().nth(1).unwrap().contains(",1.000,,2,"));
    }
}

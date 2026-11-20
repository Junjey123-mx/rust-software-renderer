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
use crate::solar::skybox::StarField;
use crate::solar::{CameraMode, SolarMeshes, SolarScene};
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

/// Builds the scene of scenario `id` at the configured size and time.
pub fn scenario_scene(id: &str, config: &SolarBenchConfig) -> Result<SolarScene, Box<dyn Error>> {
    let camera = OrbitCamera::new(Vector3::zero(), ViewPreset::Oblique);
    let view = OrbitView::for_world(
        camera,
        crate::solar::viewer::WORLD_RADIUS,
        Viewport::new(config.width, config.height, 0.1),
    )?;
    let mut s =
        SolarScene::default_system(SolarMeshes::load()?, view, Vector3::new(0.0, 1.0, 0.0))?;
    s.camera_mode = CameraMode::Perspective;
    s.clock.time = config.time;
    s.update();
    match id {
        "A" => s.bodies.clear(),
        "B" => {
            s.bodies
                .retain(|b| b.body() == crate::celestial::Body::CrimsonEntrance);
            s.rig = CameraRig::Orbit;
            s.view.camera.target = s.bodies[0].transform.position;
            s.view.camera.zoom = 2.5;
        }
        "C" => s.rig = CameraRig::Orbit,
        "D" | "H" => {}
        "E" => hover(&mut s, 1, 0.7, 0.5),
        "F" => hover(&mut s, 2, 0.7, 0.5),
        "G" => s.ship.yaw(std::f32::consts::PI),
        "I" => {
            s.stars = StarField::generate(s.stars.seed, 0);
            s.nebula.intensity = 0.0;
        }
        "J" => hover(&mut s, 1, crate::solar::flight::CLEARANCE, 0.8),
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
    for (id, name) in SCENARIOS {
        let scene = scenario_scene(id, config)?;
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
        });
    }
    Ok(rows)
}

pub fn write_solar_csv(rows: &[SolarBenchRow], path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        out,
        "scenario,name,width,height,threads,rounds,bodies,fragments_shaded,median_ms,p95_ms,min_ms,max_ms,peak_rss_kb"
    )?;
    for r in rows {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{}",
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
            r.peak_rss_kb.map_or(String::new(), |kb| kb.to_string())
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
        };
        write_solar_csv(&[row], &path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(text.starts_with(
            "scenario,name,width,height,threads,rounds,bodies,fragments_shaded,median_ms,p95_ms,min_ms,max_ms,peak_rss_kb\n"
        ));
        assert!(text.lines().nth(1).unwrap().ends_with(",1.000,"));
    }
}

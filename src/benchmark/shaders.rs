//! Laboratory 6 shader benchmark, separate from the laboratory 5 workload.
//!
//! Renders every celestial body in both filled modes with one thread and
//! with all threads, and reports the median rasterization, shading and total
//! times measured by `render_shaded` (`ShadedStats`).

use crate::celestial::{Body, CelestialScene};
use crate::pipeline::{ShadedOptions, ViewPreset};
use crate::rasterization::TriangleMode;
use crate::rendering::Framebuffer;
use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct ShaderBenchConfig {
    pub warmup: usize,
    pub rounds: usize,
    pub width: u32,
    pub height: u32,
    /// Shader threads per run; 0 means all available cores.
    pub threads: Vec<usize>,
    /// Simulated time of the measured frame.
    pub time: f32,
}

impl Default for ShaderBenchConfig {
    fn default() -> Self {
        Self {
            warmup: 1,
            rounds: 5,
            width: 800,
            height: 600,
            threads: vec![1, 0],
            time: 6.0,
        }
    }
}

/// Medians of one body, mode and thread count.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderBenchRow {
    pub body: Body,
    pub mode: TriangleMode,
    /// Actual thread count (0 resolved to the available parallelism).
    pub threads: usize,
    pub width: u32,
    pub height: u32,
    pub rounds: usize,
    pub fragments_shaded: usize,
    pub median_raster: Duration,
    pub median_shade: Duration,
    pub median_total: Duration,
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort();
    values[values.len() / 2]
}

pub fn run_shader_benchmark(
    config: &ShaderBenchConfig,
) -> Result<Vec<ShaderBenchRow>, Box<dyn Error>> {
    if config.rounds == 0 {
        return Err("the shader benchmark needs at least one round".into());
    }
    let available = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut rows = Vec::new();
    for body in Body::ALL {
        let mut scene =
            CelestialScene::new(body, ViewPreset::Oblique, config.width, config.height)?;
        scene.time = config.time;
        for mode in [
            TriangleMode::FilledBarycentric,
            TriangleMode::FilledScanline,
        ] {
            for &threads in &config.threads {
                let options = ShadedOptions {
                    threads,
                    ..ShadedOptions::default()
                };
                let mut framebuffer = Framebuffer::new(config.width, config.height);
                let mut samples = Vec::new();
                for round in 0..config.warmup + config.rounds {
                    framebuffer.clear();
                    let stats = scene.render(&mut framebuffer, mode, options)?;
                    if round >= config.warmup {
                        samples.push(stats);
                    }
                }
                rows.push(ShaderBenchRow {
                    body,
                    mode,
                    threads: if threads == 0 { available } else { threads },
                    width: config.width,
                    height: config.height,
                    rounds: config.rounds,
                    fragments_shaded: samples[0].fragments_shaded,
                    median_raster: median(samples.iter().map(|s| s.raster_time).collect()),
                    median_shade: median(samples.iter().map(|s| s.shade_time).collect()),
                    median_total: median(
                        samples
                            .iter()
                            .map(|s| s.raster_time + s.shade_time)
                            .collect(),
                    ),
                });
            }
        }
    }
    Ok(rows)
}

pub fn write_shader_csv(rows: &[ShaderBenchRow], path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        out,
        "body,mode,threads,width,height,rounds,fragments_shaded,median_raster_ms,median_shade_ms,median_total_ms"
    )?;
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    for r in rows {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{:.3},{:.3},{:.3}",
            r.body.name(),
            r.mode.name(),
            r.threads,
            r.width,
            r.height,
            r.rounds,
            r.fragments_shaded,
            ms(r.median_raster),
            ms(r.median_shade),
            ms(r.median_total)
        )?;
    }
    out.flush()?;
    Ok(())
}

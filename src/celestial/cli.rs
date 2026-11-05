//! Laboratory 6 command-line options, extracted before `parse_args` so the
//! laboratory 4/5 `CliArgs` keeps its fields and meanings.

use crate::app::{CliArgs, Scene};
use crate::celestial::DebugView;
use crate::celestial::timeline::Timeline;

/// Options that only apply to `--scene crown|entrance|house`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CelestialArgs {
    /// `--time <s>`: initial simulated time.
    pub time: Option<f32>,
    /// `--freeze-time`: time does not advance.
    pub freeze_time: bool,
    /// `--light-azimuth <deg>`.
    pub light_azimuth: Option<f32>,
    /// `--light-elevation <deg>`, in `[-89, 89]`.
    pub light_elevation: Option<f32>,
    /// `--set name=value`, repeatable; validated against the body's table.
    pub sets: Vec<(String, f32)>,
    pub debug_view: Option<DebugView>,
    /// `--threads <n>`, at least 1 (default: all cores).
    pub threads: Option<usize>,
    /// `--record <s>`: headless recording of `s` seconds to stdout.
    pub record: Option<f32>,
    /// `--fps <n>` of the recording, in `[1, 120]` (default 30).
    pub fps: Option<u32>,
    /// `--timeline <t>` of the recording (default material).
    pub timeline: Option<Timeline>,
    /// `--benchmark-shaders`: headless timing of the three shaders.
    pub benchmark_shaders: bool,
    /// `--mode` appeared on the command line (it is left for `parse_args`).
    pub mode_explicit: bool,
}

impl CelestialArgs {
    /// No laboratory 6 option was given (`--mode` is a shared option).
    pub fn is_empty(&self) -> bool {
        Self {
            mode_explicit: false,
            ..self.clone()
        } == Self::default()
    }
}

fn value<'a>(it: &mut impl Iterator<Item = &'a String>, option: &str) -> Result<&'a str, String> {
    it.next()
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn number(text: &str, option: &str) -> Result<f32, String> {
    let v: f32 = text
        .parse()
        .map_err(|_| format!("{option}: '{text}' is not a number"))?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{option}: '{text}' is not a finite number"))
    }
}

/// Consumes the laboratory 6 options and returns the remaining arguments in
/// their original order. `--mode` is detected but not consumed.
pub fn extract(args: Vec<String>) -> Result<(CelestialArgs, Vec<String>), String> {
    let mut out = CelestialArgs::default();
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--time" => {
                let t = number(value(&mut it, "--time")?, "--time")?;
                if t < 0.0 {
                    return Err(format!("--time must be at least 0 (got {t})"));
                }
                out.time = Some(t);
            }
            "--freeze-time" => out.freeze_time = true,
            "--benchmark-shaders" => out.benchmark_shaders = true,
            "--light-azimuth" => {
                out.light_azimuth = Some(number(
                    value(&mut it, "--light-azimuth")?,
                    "--light-azimuth",
                )?);
            }
            "--light-elevation" => {
                let e = number(value(&mut it, "--light-elevation")?, "--light-elevation")?;
                if !(-89.0..=89.0).contains(&e) {
                    return Err(format!("--light-elevation must be in [-89, 89] (got {e})"));
                }
                out.light_elevation = Some(e);
            }
            "--set" => {
                let text = value(&mut it, "--set")?;
                let (name, v) = text
                    .split_once('=')
                    .ok_or_else(|| format!("--set expects name=value (got '{text}')"))?;
                if name.is_empty() {
                    return Err(format!("--set expects name=value (got '{text}')"));
                }
                out.sets.push((name.to_string(), number(v, "--set")?));
            }
            "--debug-view" => {
                out.debug_view = Some(value(&mut it, "--debug-view")?.parse()?);
            }
            "--threads" => {
                let text = value(&mut it, "--threads")?;
                let n: usize = text
                    .parse()
                    .map_err(|_| format!("--threads: '{text}' is not a whole number"))?;
                if n == 0 {
                    return Err("--threads must be at least 1".into());
                }
                out.threads = Some(n);
            }
            "--record" => {
                let s = number(value(&mut it, "--record")?, "--record")?;
                if s <= 0.0 {
                    return Err(format!("--record needs a positive duration (got {s})"));
                }
                out.record = Some(s);
            }
            "--fps" => {
                let text = value(&mut it, "--fps")?;
                let n: u32 = text
                    .parse()
                    .map_err(|_| format!("--fps: '{text}' is not a whole number"))?;
                if !(1..=120).contains(&n) {
                    return Err(format!("--fps must be in [1, 120] (got {n})"));
                }
                out.fps = Some(n);
            }
            "--timeline" => {
                out.timeline = Some(value(&mut it, "--timeline")?.parse()?);
            }
            other => {
                if other == "--mode" {
                    out.mode_explicit = true;
                }
                rest.push(arg.clone());
            }
        }
    }
    Ok((out, rest))
}

/// Cross-checks the laboratory 6 options against the parsed scene.
pub fn validate(cli: &CliArgs, lab6: &CelestialArgs) -> Result<(), String> {
    let celestial = matches!(cli.scene, Scene::Celestial(_));
    if lab6.benchmark_shaders && cli.benchmark {
        return Err("--benchmark-shaders cannot be combined with --benchmark-triangles".into());
    }
    // The shader benchmark renders every body itself, so it needs no scene.
    let scene_options = CelestialArgs {
        benchmark_shaders: false,
        ..lab6.clone()
    };
    if !celestial && !scene_options.is_empty() {
        return Err(
            "--time, --freeze-time, --light-azimuth, --light-elevation, --set, --debug-view, --threads and --record require --scene crown, entrance or house"
                .into(),
        );
    }
    if lab6.record.is_none() && (lab6.fps.is_some() || lab6.timeline.is_some()) {
        return Err("--fps and --timeline require --record".into());
    }
    if lab6.record.is_some() && (cli.screenshot.is_some() || cli.interactive) {
        return Err("--record cannot be combined with --screenshot or --interactive".into());
    }
    if celestial && cli.obj.is_some() {
        return Err("--obj cannot replace the course sphere shared by the celestial scenes".into());
    }
    Ok(())
}

//! Laboratory 6 command-line options, extracted before `parse_args` so the
//! laboratory 4/5 `CliArgs` keeps its fields and meanings.

use crate::app::{CliArgs, Scene};
use crate::celestial::DebugView;
use crate::celestial::timeline::Timeline;
use crate::solar::record::SolarTimeline;
use crate::solar::skybox::{MAX_STARS, StarDensity};
use crate::solar::{CameraMode, Quality};

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
    /// `--benchmark-solar`: headless timing of the solar scenarios A-J.
    pub benchmark_solar: bool,
    /// `--camera perspective|orbit` of the solar scene.
    pub camera: Option<CameraMode>,
    /// `--stars <n>` of the solar scene (0 disables the star field).
    pub stars: Option<usize>,
    /// `--star-seed <s>` of the solar scene.
    pub star_seed: Option<u32>,
    /// `--star-density uniform|clustered` of the solar scene.
    pub star_density: Option<StarDensity>,
    /// `--nebula <0-1>`: nebula backdrop intensity of the solar scene.
    pub nebula: Option<f32>,
    /// `--solar-quality full|adaptive` of the solar scene.
    pub solar_quality: Option<Quality>,
    /// `--timeline flight` of a solar recording.
    pub solar_timeline: Option<SolarTimeline>,
    /// `--mouse-sensitivity <x>[,<y>]`: radians per pixel of the solar
    /// scene's mouse steering (one value sets both axes).
    pub mouse_sensitivity: Option<(f32, f32)>,
    /// `--invert-mouse`: moving the mouse up lowers the nose.
    pub invert_mouse: bool,
    /// `--no-culling`: the solar scene draws every object, even out of view
    /// (comparisons and tests; the image does not change).
    pub no_culling: bool,
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
            "--benchmark-solar" => out.benchmark_solar = true,
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
                let text = value(&mut it, "--timeline")?;
                match text.parse::<Timeline>() {
                    Ok(t) => out.timeline = Some(t),
                    Err(_) => out.solar_timeline = Some(text.parse()?),
                }
            }
            "--camera" => {
                out.camera = Some(value(&mut it, "--camera")?.parse()?);
            }
            "--stars" => {
                let text = value(&mut it, "--stars")?;
                let n: usize = text
                    .parse()
                    .map_err(|_| format!("--stars: '{text}' is not a whole number"))?;
                if n > MAX_STARS {
                    return Err(format!("--stars must be at most {MAX_STARS} (got {n})"));
                }
                out.stars = Some(n);
            }
            "--star-seed" => {
                let text = value(&mut it, "--star-seed")?;
                out.star_seed = Some(
                    text.parse()
                        .map_err(|_| format!("--star-seed: '{text}' is not a whole number"))?,
                );
            }
            "--star-density" => {
                out.star_density = Some(value(&mut it, "--star-density")?.parse()?);
            }
            "--nebula" => {
                let text = value(&mut it, "--nebula")?;
                let v: f32 = text
                    .parse()
                    .map_err(|_| format!("--nebula: '{text}' is not a number"))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(format!("--nebula must be between 0 and 1 (got {text})"));
                }
                out.nebula = Some(v);
            }
            "--solar-quality" => {
                out.solar_quality = Some(value(&mut it, "--solar-quality")?.parse()?);
            }
            "--mouse-sensitivity" => {
                let text = value(&mut it, "--mouse-sensitivity")?;
                let (x, y) = text.split_once(',').unwrap_or((text, text));
                let (x, y) = (
                    number(x, "--mouse-sensitivity")?,
                    number(y, "--mouse-sensitivity")?,
                );
                if x <= 0.0 || y <= 0.0 {
                    return Err(format!("--mouse-sensitivity must be positive (got {text})"));
                }
                out.mouse_sensitivity = Some((x, y));
            }
            "--invert-mouse" => out.invert_mouse = true,
            "--no-culling" => out.no_culling = true,
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
    let solar = cli.scene == Scene::Solar;
    if lab6.benchmark_shaders && cli.benchmark {
        return Err("--benchmark-shaders cannot be combined with --benchmark-triangles".into());
    }
    if lab6.benchmark_solar && (cli.benchmark || lab6.benchmark_shaders) {
        return Err("--benchmark-solar cannot be combined with another benchmark".into());
    }
    if solar && lab6.timeline.is_some() {
        return Err("--scene solar records the flight timeline (--timeline flight)".into());
    }
    if !solar && lab6.solar_timeline.is_some() {
        return Err("--timeline flight requires --scene solar".into());
    }
    if solar && cli.obj.is_some() {
        return Err("--obj cannot be combined with --scene solar".into());
    }
    if !solar && lab6.camera.is_some() {
        return Err("--camera requires --scene solar".into());
    }
    if !solar && lab6.solar_quality.is_some() {
        return Err("--solar-quality requires --scene solar".into());
    }
    if !solar && (lab6.mouse_sensitivity.is_some() || lab6.invert_mouse) {
        return Err("--mouse-sensitivity and --invert-mouse require --scene solar".into());
    }
    if !solar && lab6.no_culling {
        return Err("--no-culling requires --scene solar".into());
    }
    if !solar
        && (lab6.stars.is_some()
            || lab6.star_seed.is_some()
            || lab6.star_density.is_some()
            || lab6.nebula.is_some())
    {
        return Err(
            "--stars, --star-seed, --star-density and --nebula require --scene solar".into(),
        );
    }
    if solar {
        for (name, _) in &lab6.sets {
            if !name.contains('.') {
                return Err(format!(
                    "--set '{name}' needs a body prefix in --scene solar (crown., entrance. or house.)"
                ));
            }
        }
    }
    // The shader benchmark renders every body itself, so it needs no scene;
    // the solar scene shares `--threads`, `--time` and prefixed `--set` with
    // the single-body scenes, and the solar benchmark takes `--threads`.
    let scene_options = CelestialArgs {
        benchmark_shaders: false,
        benchmark_solar: false,
        threads: if solar || lab6.benchmark_solar {
            None
        } else {
            lab6.threads
        },
        time: if solar { None } else { lab6.time },
        sets: if solar { Vec::new() } else { lab6.sets.clone() },
        camera: None,
        stars: None,
        star_seed: None,
        star_density: None,
        nebula: None,
        solar_quality: None,
        solar_timeline: None,
        mouse_sensitivity: None,
        invert_mouse: false,
        no_culling: false,
        record: if solar { None } else { lab6.record },
        fps: if solar { None } else { lab6.fps },
        ..lab6.clone()
    };
    if !celestial && !scene_options.is_empty() {
        return Err(
            "--time, --freeze-time, --light-azimuth, --light-elevation, --set, --debug-view, --threads and --record require --scene crown, entrance or house"
                .into(),
        );
    }
    if lab6.record.is_none()
        && (lab6.fps.is_some() || lab6.timeline.is_some() || lab6.solar_timeline.is_some())
    {
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

//! Application lifecycle and orchestration.

use crate::benchmark::{
    BenchmarkConfig, build_workload, profile_workload, run_benchmark, write_csv,
};
use crate::config::AppConfig;
use crate::geometry::{SCENE_HEIGHT, SCENE_WIDTH, Vertex, triangle_scene};
use crate::io::load_obj;
use crate::pipeline::{
    OrbitCamera, OrbitView, Projection, Uniforms, ViewPreset, Viewport, model_bounds,
    render_interactive, render_with_mode,
};
use crate::rasterization::TriangleMode;
use crate::rendering::{Framebuffer, Presenter};
use raylib::consts::KeyboardKey;
use raylib::math::Vector3;
use std::error::Error;
use std::path::PathBuf;

/// Command-line options.
///
/// * `--obj <file>`: render another triangulated OBJ instead of the ship.
/// * `--screenshot <file>`: save the first presented frame of the real
///   window to `file` and exit.
/// * `--mode wireframe|barycentric|scanline`: triangle rasterization mode
///   (default: wireframe, the laboratory 4 behaviour).
/// * `--scene ship|triangles|crown|entrance|house`: the OBJ model (default), the controlled
///   triangle scene used to compare the modes, or a laboratory 6 celestial
///   body (its own options are extracted first by `celestial::cli::extract`).
/// * `--benchmark-triangles`: headless timing of both fill algorithms; no
///   window is created.
/// * `--benchmark-output <dir>`: where `benchmark_raw.csv` and
///   `benchmark_summary.csv` are written (default `docs/results/lab5`).
/// * `--profile [--profile-mode barycentric|scanline] [--profile-iterations N]`:
///   with `--benchmark-triangles`, run the workload repeatedly without timing
///   so an external profiler can sample it. Default: both fills, 300 each.
/// * `--interactive`: orbit camera around the ship with keyboard controls
///   (arrows orbit/tilt, Q/E zoom, 1/2/3 mode, R reset, T top).
/// * `--view oblique|top|front|side`: starting orbit preset (implies the
///   orbit projection; add `--interactive` for keyboard control).
/// * `--help`: print this summary and exit.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub obj: Option<PathBuf>,
    pub screenshot: Option<String>,
    pub mode: TriangleMode,
    pub scene: Scene,
    pub benchmark: bool,
    pub benchmark_output: Option<PathBuf>,
    pub profile: bool,
    pub profile_mode: Option<TriangleMode>,
    pub profile_iterations: Option<usize>,
    pub interactive: bool,
    pub view: Option<ViewPreset>,
    pub help: bool,
}

pub const USAGE: &str = "\
Usage: rust-software-renderer [options]
  --scene ship|triangles         model (default ship) or the Lab 5 triangle scene
  --mode wireframe|barycentric|scanline   rasterization mode (default wireframe)
  --obj <file>                   another triangulated OBJ (scene ship only)
  --screenshot <file>            save the first frame and exit
  --interactive                  orbit camera with keyboard control (scene ship)
  --view oblique|top|front|side  starting camera preset (scene ship)
  --benchmark-triangles [--benchmark-output <dir>]   headless timing, CSV output
  --benchmark-triangles --profile [--profile-mode m] [--profile-iterations N]
  --help                         this text
  --scene crown|entrance|house   Lab 6 celestial bodies on the course sphere (default mode scanline)
  --time <s> --freeze-time --light-azimuth <deg> --light-elevation <deg>
  --set <param>=<value>   --debug-view <view>   --threads <n>
Interactive keys: arrows orbit/tilt, Q/E zoom, 1/2/3 mode, R reset, T top, F front, S side, ESC quit
Celestial keys: TAB body, SPACE pause, J/L I/K light, N debug view, [ ] main parameter, C/H/B/P toggle clouds, shadows, bump and parallax when the body provides them";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scene {
    #[default]
    Ship,
    Triangles,
    /// Laboratory 6: a celestial body on the course sphere.
    Celestial(crate::celestial::Body),
}

impl std::str::FromStr for Scene {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ship" => Ok(Self::Ship),
            "triangles" => Ok(Self::Triangles),
            other => other.parse().map(Self::Celestial).map_err(|_| {
                format!(
                    "unknown scene '{other}' (expected ship, triangles, crown, entrance or house)"
                )
            }),
        }
    }
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliArgs, String> {
    let mut out = CliArgs::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--obj" => {
                out.obj = Some(PathBuf::from(
                    it.next().ok_or("--obj requires a file path")?,
                ));
            }
            "--screenshot" => {
                out.screenshot = Some(it.next().ok_or("--screenshot requires a file name")?);
            }
            "--benchmark-triangles" => out.benchmark = true,
            "--profile" => out.profile = true,
            "--interactive" => out.interactive = true,
            "--help" | "-h" => out.help = true,
            "--view" => {
                out.view = Some(
                    it.next()
                        .ok_or("--view requires oblique, top, front or side")?
                        .parse()?,
                );
            }
            "--profile-mode" => {
                let mode: TriangleMode = it
                    .next()
                    .ok_or("--profile-mode requires barycentric or scanline")?
                    .parse()?;
                if mode == TriangleMode::Wireframe {
                    return Err(
                        "--profile-mode expects a filled mode (barycentric or scanline)".into(),
                    );
                }
                out.profile_mode = Some(mode);
            }
            "--profile-iterations" => {
                let n: usize = it
                    .next()
                    .ok_or("--profile-iterations requires a number")?
                    .parse()
                    .map_err(|e| format!("--profile-iterations: {e}"))?;
                if n == 0 {
                    return Err("--profile-iterations must be at least 1".into());
                }
                out.profile_iterations = Some(n);
            }
            "--benchmark-output" => {
                out.benchmark_output = Some(PathBuf::from(
                    it.next().ok_or("--benchmark-output requires a directory")?,
                ));
            }
            "--scene" => {
                out.scene = it
                    .next()
                    .ok_or("--scene requires ship or triangles")?
                    .parse()?;
            }
            "--mode" => {
                out.mode = it
                    .next()
                    .ok_or("--mode requires wireframe, barycentric or scanline")?
                    .parse()?;
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(out)
}

/// Camera commands produced by the keyboard. Continuous actions are scaled
/// by the frame delta; discrete ones fire once per key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAction {
    OrbitLeft,
    OrbitRight,
    TiltUp,
    TiltDown,
    ZoomIn,
    ZoomOut,
    Reset,
    View(ViewPreset),
}

/// Radians per second for orbit and tilt.
pub const ORBIT_SPEED: f32 = 1.6;
/// Zoom factor per second (exponential, so in/out are symmetric).
pub const ZOOM_SPEED: f32 = 2.0;
/// Longest frame delta applied, so a stall never teleports the camera.
pub const MAX_DT: f32 = 0.05;

/// Applies one action to the camera. Pure: no window, no raylib.
pub fn apply_input(camera: &mut OrbitCamera, action: CameraAction, dt: f32) {
    let dt = if dt.is_finite() {
        dt.clamp(0.0, MAX_DT)
    } else {
        0.0
    };
    match action {
        CameraAction::OrbitLeft => camera.orbit(-ORBIT_SPEED * dt),
        CameraAction::OrbitRight => camera.orbit(ORBIT_SPEED * dt),
        CameraAction::TiltUp => camera.tilt(ORBIT_SPEED * dt),
        CameraAction::TiltDown => camera.tilt(-ORBIT_SPEED * dt),
        CameraAction::ZoomIn => camera.zoom_by(ZOOM_SPEED.powf(dt)),
        CameraAction::ZoomOut => camera.zoom_by(ZOOM_SPEED.powf(-dt)),
        CameraAction::Reset => camera.reset(),
        CameraAction::View(preset) => camera.set_preset(preset),
    }
}

/// Reads the keyboard state of this frame into camera actions and an
/// optional mode change.
pub(crate) fn poll_input(rl: &raylib::RaylibHandle) -> (Vec<CameraAction>, Option<TriangleMode>) {
    let held = [
        (KeyboardKey::KEY_LEFT, CameraAction::OrbitLeft),
        (KeyboardKey::KEY_RIGHT, CameraAction::OrbitRight),
        (KeyboardKey::KEY_UP, CameraAction::TiltUp),
        (KeyboardKey::KEY_DOWN, CameraAction::TiltDown),
        (KeyboardKey::KEY_E, CameraAction::ZoomIn),
        (KeyboardKey::KEY_Q, CameraAction::ZoomOut),
    ];
    let pressed = [
        (KeyboardKey::KEY_R, CameraAction::Reset),
        (KeyboardKey::KEY_T, CameraAction::View(ViewPreset::Top)),
        (KeyboardKey::KEY_F, CameraAction::View(ViewPreset::Front)),
        (KeyboardKey::KEY_S, CameraAction::View(ViewPreset::Side)),
    ];
    let mut actions: Vec<CameraAction> = held
        .iter()
        .filter(|(k, _)| rl.is_key_down(*k))
        .map(|(_, a)| *a)
        .collect();
    actions.extend(
        pressed
            .iter()
            .filter(|(k, _)| rl.is_key_pressed(*k))
            .map(|(_, a)| *a),
    );
    let mode = [
        (KeyboardKey::KEY_ONE, TriangleMode::Wireframe),
        (KeyboardKey::KEY_TWO, TriangleMode::FilledBarycentric),
        (KeyboardKey::KEY_THREE, TriangleMode::FilledScanline),
    ]
    .iter()
    .find(|(k, _)| rl.is_key_pressed(*k))
    .map(|(_, m)| *m);
    (actions, mode)
}

/// Raylib writes screenshots as `<working dir>/<file name>`, ignoring any
/// directory; move the file to the path the user actually asked for. Copy
/// then remove, so a target on another filesystem works too.
pub(crate) fn save_screenshot(
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
    file: &str,
) -> Result<(), Box<dyn Error>> {
    let target = std::path::Path::new(file);
    let name = target.file_name().ok_or("--screenshot needs a file name")?;
    rl.take_screenshot(thread, &name.to_string_lossy());
    if target.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
        std::fs::copy(name, target)?;
        std::fs::remove_file(name)?;
    }
    Ok(())
}

/// Headless profiling workload: no window, no timer, just rasterization.
fn run_profile_cli(mode: Option<TriangleMode>, iterations: usize) -> Result<(), Box<dyn Error>> {
    let workload = build_workload();
    let modes: Vec<TriangleMode> = match mode {
        Some(m) => vec![m],
        None => vec![
            TriangleMode::FilledBarycentric,
            TriangleMode::FilledScanline,
        ],
    };
    println!(
        "Profiling workload: {} triangles, {} iterations per mode, modes {:?}",
        workload.triangles.len(),
        iterations,
        modes.iter().map(|m| m.name()).collect::<Vec<_>>()
    );
    for m in modes {
        let total = profile_workload(&workload, m, iterations);
        println!("{m}: {total} fragments generated in {iterations} iterations");
    }
    Ok(())
}

/// Headless benchmark: measured before any raylib initialisation.
fn run_benchmark_cli(output_dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let workload = build_workload();
    let config = BenchmarkConfig::default();
    println!(
        "Benchmark: {} triangles, {}x{}, {} warm-up rounds, {} rounds x {} iterations",
        workload.triangles.len(),
        workload.bounds.width,
        workload.bounds.height,
        config.warmup_rounds,
        config.rounds,
        config.iterations
    );
    let report = run_benchmark(&workload, config)?;
    println!(
        "Fragments per iteration (both fills, parity verified): {}",
        report.fragments_per_iteration
    );
    for r in &report.rounds {
        println!(
            "{:<12} round {} : {:>12} ns total, {:>10.1} ns/iteration",
            r.mode.to_string(),
            r.round,
            r.elapsed_ns,
            r.ns_per_iteration()
        );
    }
    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let med = report.median_round_ns(mode);
        println!(
            "{:<12} median of {} rounds: {:.0} ns/round = {:.1} ns/iteration",
            mode.to_string(),
            config.rounds,
            med,
            med / config.iterations as f64
        );
    }
    let raw = output_dir.join("benchmark_raw.csv");
    let summary = output_dir.join("benchmark_summary.csv");
    write_csv(&report, &raw, &summary)?;
    println!("Wrote {} and {}", raw.display(), summary.display());
    Ok(())
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (lab6, rest) = crate::celestial::cli::extract(raw)?;
    let args = parse_args(rest)?;
    if args.help {
        println!("{USAGE}");
        return Ok(());
    }
    crate::celestial::cli::validate(&args, &lab6)?;
    if args.profile && !args.benchmark {
        return Err("--profile requires --benchmark-triangles".into());
    }
    if args.benchmark && args.profile {
        return run_profile_cli(args.profile_mode, args.profile_iterations.unwrap_or(300));
    }
    if args.benchmark {
        let dir = args
            .benchmark_output
            .unwrap_or_else(|| PathBuf::from("docs/results/lab5"));
        return run_benchmark_cli(&dir);
    }
    if let Scene::Celestial(body) = args.scene {
        return crate::celestial::viewer::run(body, &args, &lab6);
    }
    let config = AppConfig::default();
    let mode = args.mode;
    let (vertex_array, projection, width, height) = match args.scene {
        Scene::Ship => {
            let model_path = args.obj.unwrap_or(config.model_path.clone());
            // Load once at start-up; the mesh does not change between frames.
            let mesh = load_obj(&model_path)?;
            println!(
                "Loaded {}: {} vertices, {} indices, {} triangles",
                model_path.display(),
                mesh.vertices.len(),
                mesh.indices.len(),
                mesh.triangle_count()
            );
            let vertices = mesh.expand_vertices(Vertex::DEFAULT_COLOR);
            let projection = if args.interactive || args.view.is_some() {
                let bounds = model_bounds(&vertices).ok_or("model has no finite geometry")?;
                let camera = OrbitCamera::new(bounds.center, args.view.unwrap_or_default());
                let viewport = Viewport::new(config.width, config.height, config.margin_ratio);
                Projection::Orbit(OrbitView::new(camera, &bounds, viewport)?)
            } else {
                Projection::Fit {
                    plane: config.view_plane,
                    margin_ratio: config.margin_ratio,
                }
            };
            (vertices, projection, config.width, config.height)
        }
        Scene::Triangles => {
            if args.obj.is_some() {
                return Err("--obj cannot be combined with --scene triangles".into());
            }
            if args.interactive || args.view.is_some() {
                return Err("--interactive and --view require --scene ship".into());
            }
            let scene = triangle_scene();
            println!(
                "Loaded triangle scene: {} triangles in screen space",
                scene.len() / 3
            );
            // Already in pixels: no viewport fit, no double transform.
            (scene, Projection::Screen, SCENE_WIDTH, SCENE_HEIGHT)
        }
        Scene::Celestial(_) => unreachable!("celestial scenes return early"),
    };
    let uniforms = Uniforms::default();

    let title = if matches!(projection, Projection::Orbit(_)) {
        "Rust Software Renderer - Lab 5 Interactive"
    } else {
        config.window_title
    };
    if args.interactive {
        println!(
            "Controls: arrows orbit/tilt, Q/E zoom, 1/2/3 wireframe/barycentric/scanline, R reset, T top, F front, S side, ESC quit"
        );
    }
    let (mut rl, thread) = raylib::init()
        .size(width as i32, height as i32)
        .title(title)
        .build();
    rl.set_target_fps(config.target_fps);

    let mut framebuffer = Framebuffer::new(width, height);
    framebuffer.set_background_color(Vector3::new(0.05, 0.05, 0.08));
    let mut presenter = Presenter::new(&mut rl, &thread, &framebuffer)?;

    let mut mode = mode;
    let mut projection = projection;
    let mut reported = false;
    while !rl.window_should_close() {
        if let (Projection::Orbit(view), true) = (&mut projection, args.interactive) {
            let dt = rl.get_frame_time();
            let (actions, new_mode) = poll_input(&rl);
            for action in actions {
                apply_input(&mut view.camera, action, dt);
            }
            if let Some(m) = new_mode
                && m != mode
            {
                mode = m;
                println!("Mode: {mode} (camera unchanged)");
            }
        }
        framebuffer.clear();
        let stats = match projection {
            Projection::Orbit(view) => {
                render_interactive(&mut framebuffer, &uniforms, &vertex_array, view, mode)?
            }
            other => render_with_mode(&mut framebuffer, &uniforms, &vertex_array, other, mode)?,
        };
        if !reported {
            println!(
                "Rendered {} triangles into {} fragments (mode {}, {:?}, {}x{} @ {} FPS)",
                stats.triangles_processed,
                stats.fragments_generated,
                mode,
                projection,
                width,
                height,
                config.target_fps
            );
            reported = true;
        }
        presenter.present(&mut rl, &thread, &framebuffer)?;
        if let Some(file) = &args.screenshot {
            save_screenshot(&mut rl, &thread, file)?;
            println!("Saved window screenshot to {file}");
            break;
        }
    }
    Ok(())
}

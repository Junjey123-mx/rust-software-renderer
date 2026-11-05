//! Headless benchmarking of fragment generation (laboratory 5).
//!
//! Nothing here opens a window or touches the framebuffer; only the stage-3
//! rasterizer is exercised and timed. `shaders` (laboratory 6) is a separate
//! benchmark of the procedural shaders that leaves this workload untouched.

pub mod shaders;
pub mod triangles;

pub use shaders::{ShaderBenchConfig, ShaderBenchRow, run_shader_benchmark, write_shader_csv};

pub use triangles::{
    BenchmarkConfig, BenchmarkReport, Round, Workload, build_workload, median_ns, profile_workload,
    run_benchmark, time_mode, verify_parity, write_csv,
};

//! Headless benchmarking of fragment generation (laboratory 5).
//!
//! Nothing here opens a window or touches the framebuffer; only the stage-3
//! rasterizer is exercised and timed.

pub mod triangles;

pub use triangles::{
    BenchmarkConfig, BenchmarkReport, Round, Workload, build_workload, median_ns, run_benchmark,
    time_mode, verify_parity, write_csv,
};

use std::process::ExitCode;

fn main() -> ExitCode {
    match rust_software_renderer::app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

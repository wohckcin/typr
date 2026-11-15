//! Runs the NSH app on this machine's desktop backend (`cargo run --bin typr_nsh`).
//!
//! The Rune binary (`src/main.rs`) stays until the port is merged; this is the NSH entry point.

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;

fn main() -> std::process::ExitCode {
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<typr::shell::App>()),
        platform::AppOptions {
            title: "typr".to_owned(),
            app_id: "dev.typr.app".to_owned(),
            size: (960, 640),
            ..Default::default()
        },
    )
}

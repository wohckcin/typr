//! Runs the app on this machine's desktop backend (`cargo run -p typr`).

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;

fn main() -> std::process::ExitCode {
    // The log is decided before anything can fail: the level from `TYPR_LOG`, the file in the app's
    // data folder. A quit flushes it after the platform's loop returns.
    match typr::paths::data_root() {
        Ok(root) => match typr::logging::install(&typr::paths::logs_root(&root), typr::logging::level_from_env()) {
            Ok(path) => log::info!("typr starts; log at {}", path.display()),
            Err(error) => eprintln!("typr: the log could not be installed: {error}"),
        },
        Err(error) => eprintln!("typr: no data folder: {error}"),
    }
    let code = platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<typr::shell::App>()),
        platform::AppOptions {
            title: "typr".to_owned(),
            app_id: "dev.typr.app".to_owned(),
            size: (960, 640),
            ..Default::default()
        },
    );
    typr::logging::shutdown();
    code
}

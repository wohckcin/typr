//! Runs the app on this machine's desktop backend (`cargo run -p typr`).

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;

fn main() -> std::process::ExitCode {
    // The data folder and the log belong to the app: both need the engine's runtime, so `App`
    // resolves them once it is mounted. A quit flushes the log after the platform's loop returns.
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
    finish(code)
}

/// The platform's exit status as this process's: the GTK host answers with GLib's code, which is
/// the same number in a different type.
#[cfg(target_os = "macos")]
fn finish(code: std::process::ExitCode) -> std::process::ExitCode {
    code
}

#[cfg(target_os = "linux")]
fn finish(code: nsh_platform_linux::gtk4::glib::ExitCode) -> std::process::ExitCode {
    std::process::ExitCode::from(code.get())
}

//! The visual language is enforced by the same check CI runs: a literal or a base control that
//! creeps into a screen fails here first.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

struct Check {
    ok: bool,
    report: String,
}

fn check(files: &[&Path]) -> Check {
    let script = workspace().join("scripts/check-tokens.sh");
    let mut command = Command::new("bash");
    command.arg(&script).current_dir(workspace());
    for file in files {
        command.arg(file);
    }
    let output = command.output().expect("the check runs");
    Check {
        ok: output.status.success(),
        report: String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout),
    }
}

fn planted(source: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("screen.rs");
    std::fs::write(&file, source).expect("the screen");
    (dir, file)
}

#[test]
fn every_screen_draws_with_the_tokens() {
    let result = check(&[]);
    assert!(result.ok, "a screen writes a literal or a base control:\n{}", result.report);
}

#[test]
fn a_pixel_literal_is_refused() {
    let (_dir, file) = planted("fn view() {\n    view().h(px(24.0));\n}\n");
    let result = check(&[&file]);
    assert!(!result.ok, "the check let a pixel literal through");
    assert!(result.report.contains("px(24.0)"), "{}", result.report);
}

#[test]
fn a_colour_literal_is_refused() {
    let (_dir, file) = planted("fn view() {\n    view().bg(rgb(0x1e1e1e));\n}\n");
    let result = check(&[&file]);
    assert!(!result.ok, "the check let a colour literal through");
    assert!(result.report.contains("rgb(0x1e1e1e)"), "{}", result.report);
}

#[test]
fn a_base_control_import_is_refused() {
    let (_dir, file) = planted("use nsh_base::button;\nfn view() {}\n");
    let result = check(&[&file]);
    assert!(!result.ok, "the check let a base control through");
    assert!(result.report.contains("use nsh_base::button"), "{}", result.report);
}

#[test]
fn the_theme_import_is_allowed() {
    let (_dir, file) = planted("use nsh_base::theme;\nfn view() { let _ = theme(); }\n");
    let result = check(&[&file]);
    assert!(result.ok, "the check refused the theme: {}", result.report);
}

#[test]
fn a_missing_screen_is_reported_rather_than_skipped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("gone.rs");
    let result = check(&[&file]);
    assert!(!result.ok, "a missing file passed the check");
    assert!(result.report.contains("missing"), "{}", result.report);
}

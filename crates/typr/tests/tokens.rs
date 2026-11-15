//! The visual language is only as good as its enforcement: this runs the same check CI runs, so a
//! literal that creeps back into a screen fails here first.

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

#[test]
fn every_screen_draws_with_the_tokens() {
    let result = check(&[]);
    assert!(result.ok, "a screen writes a literal:\n{}", result.report);
}

#[test]
fn a_reintroduced_spacing_literal_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("screen.rs");
    std::fs::write(&file, "fn view() {\n    col().padding(24.0);\n}\n").expect("the screen");
    let result = check(&[&file]);
    assert!(!result.ok, "the check let a padding literal through");
    assert!(result.report.contains("padding(24.0)"), "{}", result.report);
}

#[test]
fn a_font_size_literal_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("screen.rs");
    std::fs::write(
        &file,
        "fn view() {\n    text(\"x\").font(type_::BODY.lh(19.0));\n}\n",
    )
    .expect("the screen");
    let result = check(&[&file]);
    assert!(!result.ok, "the check let a line height through");
    assert!(result.report.contains("lh(19.0)"), "{}", result.report);
}

#[test]
fn a_missing_screen_is_reported_rather_than_skipped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("gone.rs");
    let result = check(&[&file]);
    assert!(!result.ok, "a missing file passed the check");
    assert!(result.report.contains("missing"), "{}", result.report);
}

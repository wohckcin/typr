use std::path::Path;

use super::Attachments;
use typr_core::transfer::attach;

fn root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(dir.join(name), bytes).unwrap();
}

#[test]
fn a_reference_inside_the_directory_resolves() {
    let dir = root();
    write(dir.path(), "diagram.png", b"png");
    let attachments = Attachments::detached(dir.path().to_path_buf());

    let resolved = attachments.resolve("diagram.png").expect("it is there");
    assert!(resolved.ends_with("diagram.png"));
}

#[test]
fn a_path_escaping_the_directory_is_refused() {
    let dir = root();
    let outside = dir.path().parent().unwrap().join("outside.png");
    std::fs::write(&outside, b"png").unwrap();
    let attachments = Attachments::detached(dir.path().to_path_buf());

    for src in [
        "../outside.png",
        "sub/../../outside.png",
        "/etc/passwd",
        "~/secret.png",
    ] {
        assert_eq!(attachments.resolve(src), None, "{src} must not resolve");
    }
    assert_eq!(attachments.resolve(""), None);
    assert_eq!(
        attachments.resolve("missing.png"),
        None,
        "a missing file is not resolved"
    );
}

#[test]
fn attaching_copies_the_file_and_names_it_without_overwriting() {
    let dir = root();
    let source_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("diagram.png");
    std::fs::write(&source, b"bytes").unwrap();

    let first = attach(dir.path(), &source).unwrap();
    let second = attach(dir.path(), &source).unwrap();

    assert_eq!(first, "diagram.png");
    assert_eq!(
        second, "diagram-2.png",
        "the second copy does not overwrite the first"
    );
    assert_eq!(std::fs::read(dir.path().join(&first)).unwrap(), b"bytes");
    let attachments = Attachments::detached(dir.path().to_path_buf());
    assert!(attachments.resolve(&second).is_some());
}

#[test]
fn a_detached_directory_holds_nothing() {
    let attachments = Attachments::detached(std::path::PathBuf::new());
    assert_eq!(attachments.resolve("diagram.png"), None);
}

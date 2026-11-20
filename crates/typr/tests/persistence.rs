//! What the shell writes to the library on disk when a screen reports a change.

use std::sync::Mutex;

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::store::Db;

/// `TYPR_DATA_DIR` is process-wide, so the tests that point the app at a library take turns.
static DATA_DIR: Mutex<()> = Mutex::new(());

fn library(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-persist-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    Db::open(&root.join("typr.sqlite3")).unwrap();
    root
}

fn start(root: &std::path::Path) -> Headless {
    unsafe { std::env::set_var("TYPR_DATA_DIR", root) };
    Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap()
}

#[test]
fn an_applied_new_card_cap_is_stored() -> Result<(), String> {
    let _turn = DATA_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = library("cap");
    let mut app = start(&root);
    app.wait_for("Settings")?;
    app.press("Settings")?;
    app.wait_for("role:button[Apply]")?;
    app.clear("text:20")?;
    app.type_text("text: >> nth=1", "35")?;
    app.press("role:button[Apply]")?;
    app.settle();
    let stored = Db::open(&root.join("typr.sqlite3"))
        .unwrap()
        .settings()
        .unwrap()
        .new_per_day;
    assert_eq!(stored, 35);
    Ok(())
}

#[test]
fn an_imported_archive_shows_in_the_sidebar_without_a_restart() -> Result<(), String> {
    let _turn = DATA_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = library("import");
    let mut app = start(&root);
    app.on_pick_paths(vec![
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/mochi-import.mochi"),
    ])?;
    app.wait_for("Import")?;
    app.press("Import")?;
    app.wait_for("Imported 1")?;
    Ok(())
}

//! The settings screen: a valid cap is accepted, an out-of-range one is refused with its limit and
//! the old value stays. Persistence is out of scope and is not tested here.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use std::path::PathBuf;

use typr::screens::settings::{LibraryInfo, Values, check_new_card_cap, settings};

fn library() -> LibraryInfo {
    let mut info = LibraryInfo::default();
    info.database = PathBuf::from("/tmp/typr/typr.sqlite3");
    info.schema = 12;
    info.bytes = 4096;
    info.attachments = PathBuf::from("/tmp/typr/attachments");
    info.logs = PathBuf::from("/tmp/typr/logs");
    info
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let values = Values::new(20);
    view()
        .child(settings(values, library(), |_| {}, || {}, || {}, || {}))
        .child(text(move || format!("cap={}", values.cap.get())))
}

#[test]
fn the_screen_reports_where_the_library_is() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/typr.sqlite3/")?;
    app.wait_for("text:/attachments/")?;
    app.wait_for(r"text:/\/tmp\/typr\/logs/")?;
    app.wait_for("text:/Schema 12/")?;
    app.wait_for("text:/4096 bytes/")?;
    // Each path offers a control that shows it in the file manager.
    app.wait_for("role:button[Show Database in the file manager]")?;
    app.wait_for("role:button[Show Attachments in the file manager]")?;
    app.wait_for("role:button[Show Log in the file manager]")?;
    Ok(())
}

#[test]
fn the_screen_offers_import_and_export() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Import]")?;
    app.wait_for("role:button[Export]")?;
    Ok(())
}

#[test]
fn an_out_of_range_cap_is_refused_and_the_old_value_stays() {
    assert_eq!(
        check_new_card_cap("100000"),
        Err("New cards a day must be 1000 or less".to_string())
    );
    assert!(check_new_card_cap("abc").is_err());
    assert_eq!(check_new_card_cap(" 25 "), Ok(25));
}

#[test]
fn a_refused_cap_names_its_limit_on_screen() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text: >> nth=1", "100000")?;
    app.press("role:button[Apply]")?;
    app.wait_for("text:/1000 or less/")?;
    app.wait_for("text:/cap=20/")?;
    Ok(())
}

#[test]
fn an_accepted_cap_takes_effect_at_once() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text: >> nth=1", "35")?;
    app.press("role:button[Apply]")?;
    app.wait_for("text:/cap=35/")?;
    Ok(())
}

#[test]
fn an_accepted_cap_persists_across_a_restart() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let db = typr_core::store::Db::open(&path).expect("open");
    db.apply(
        &typr_core::store::Command::SetSetting {
            key: typr_core::store::SettingKey::NewPerDay,
            value: typr_core::store::SettingValue::Number(35),
        },
        0,
    )
    .expect("store cap");
    drop(db);

    let reopened = typr_core::store::Db::open(&path).expect("reopen");
    assert_eq!(reopened.settings().expect("settings").new_per_day, 35);
}

/// An empty library on disk, with `theme` stored when given.
fn library_with_theme(name: &str, theme: Option<typr_core::store::Theme>) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-settings-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    if let Some(theme) = theme {
        db.apply(
            &typr_core::store::Command::SetSetting {
                key: typr_core::store::SettingKey::Theme,
                value: typr_core::store::SettingValue::Theme(theme),
            },
            0,
        )
        .unwrap();
    }
    root
}

#[test]
fn the_theme_control_writes_the_stored_theme() -> Result<(), String> {
    let root = library_with_theme("control", None);
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("Settings")?;
    app.press("Settings")?;
    app.wait_for("role:button[Light]")?;
    app.press("role:button[Light]")?;
    app.settle();
    // Dropping the app joins the store worker so the queued write has landed before reopening.
    drop(app);
    let stored = typr_core::store::Db::open(&root.join("typr.sqlite3"))
        .unwrap()
        .settings()
        .unwrap()
        .theme;
    assert_eq!(stored, typr_core::store::Theme::Light);
    Ok(())
}

#[test]
fn a_row_matches_by_a_case_insensitive_part_of_its_label() {
    use typr::screens::settings::row_matches;
    assert!(
        row_matches("New cards a day", ""),
        "an empty query shows every row"
    );
    assert!(row_matches("New cards a day", "   "), "so does a blank one");
    assert!(row_matches("New cards a day", "CARDS"), "case is ignored");
    assert!(
        row_matches("New cards a day", "s a d"),
        "any part of the label"
    );
    assert!(!row_matches("Theme", "cards"));
}

#[test]
fn typing_a_name_hides_the_other_rows() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("New cards a day")?;
    app.type_text("Find a setting", "theme")?;
    app.wait_for_gone("New cards a day", std::time::Duration::from_secs(5))?;
    assert!(app.screen().shows("Theme"), "the matching row stays");
    assert!(!app.screen().shows("Database"), "the others go");
    Ok(())
}

#[test]
fn a_query_nothing_matches_says_so() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Theme")?;
    app.type_text("Find a setting", "zzzz")?;
    app.wait_for("No setting matches")?;
    assert!(!app.screen().shows("Theme"));
    Ok(())
}

#[test]
fn clearing_the_query_restores_the_rows() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Theme")?;
    app.type_text("Find a setting", "theme")?;
    app.wait_for_gone("New cards a day", std::time::Duration::from_secs(5))?;
    app.clear("text:theme")?;
    app.wait_for("New cards a day")?;
    app.wait_for("Theme")?;
    Ok(())
}

#[test]
fn the_screen_accepts_exactly_the_caps_the_store_keeps() {
    use typr_core::store::{SettingKey, SettingValue};
    for cap in [0_u32, 1, 999, 1_000, 1_001, 10_000] {
        assert_eq!(
            check_new_card_cap(&cap.to_string()).is_ok(),
            SettingKey::NewPerDay
                .validate(&SettingValue::Number(cap))
                .is_ok(),
            "a cap of {cap}"
        );
    }
}

#[test]
fn the_screen_offers_no_review_counts_switch_that_nothing_reads() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Theme")?;
    assert!(!app.screen().shows("Show review counts"));
    Ok(())
}

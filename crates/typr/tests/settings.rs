//! The settings screen: a valid cap is accepted, an out-of-range one is refused with its limit and
//! the old value stays. Persistence is out of scope and is not tested here.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use std::path::PathBuf;

use typr::screens::settings::{
    LibraryInfo, NEW_CARD_CAP_MAX, Values, check_new_card_cap, settings,
};

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
    let values = Values::new(20, true);
    view()
        .child(settings(values, library(), |_| {}, || {}, || {}))
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
        Err(format!(
            "New cards a day must be {NEW_CARD_CAP_MAX} or less"
        ))
    );
    assert!(check_new_card_cap("abc").is_err());
    assert_eq!(check_new_card_cap(" 25 "), Ok(25));
}

#[test]
fn a_refused_cap_names_its_limit_on_screen() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text:", "100000")?;
    app.press("role:button[Apply]")?;
    app.wait_for("text:/10000 or less/")?;
    app.wait_for("text:/cap=20/")?;
    Ok(())
}

#[test]
fn an_accepted_cap_takes_effect_at_once() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text:", "35")?;
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

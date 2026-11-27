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
    // Dropping the app joins the store worker so the queued write has landed before reopening.
    drop(app);
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
    // Import lives in Settings.
    app.wait_for("Settings")?;
    app.press("Settings")?;
    app.wait_for("role:button[Import]")?;
    app.press("role:button[Import]")?;
    app.wait_for("Imported 1")?;
    Ok(())
}

/// A library with one card, "gato", in a deck of its own.
fn library_with_card(name: &str) -> (std::path::PathBuf, i64) {
    use typr_core::store::{Command, Outcome};
    let root = library(name);
    let db = Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: "Spanish".to_owned(),
            },
            0,
        )
        .unwrap()
    {
        Outcome::Deck(id) => id,
        other => panic!("deck: {other:?}"),
    };
    let card = match db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: "gato\n---\ncat".to_owned(),
            },
            0,
        )
        .unwrap()
    {
        Outcome::Card(id) => id,
        other => panic!("card: {other:?}"),
    };
    (root, card)
}

#[test]
fn a_refused_save_leaves_the_card_unsaved_and_says_why() -> Result<(), String> {
    use typr_core::store::Command;
    let _turn = DATA_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let (root, card) = library_with_card("refused");
    let mut app = start(&root);
    app.wait_for("role:button[gato]")?;
    app.press("role:button[gato]")?;
    app.wait_for("Card source")?;
    // The card is emptied from the trash behind the open editor, so the store refuses the save.
    let other = Db::open(&root.join("typr.sqlite3")).unwrap();
    other.apply(&Command::TrashCard { id: card }, 0).unwrap();
    other.apply(&Command::EmptyTrash, 0).unwrap();
    app.type_text("label:Card source", "!")?;
    app.press("role:button[Save]")?;
    app.settle();
    assert!(app.screen().shows("Unsaved"), "a refused save is not Saved");
    assert!(!app.screen().shows("Saved"));
    app.wait_for("text:/not saved/i")?;
    Ok(())
}

#[test]
fn an_accepted_save_reads_saved_and_is_stored() -> Result<(), String> {
    let _turn = DATA_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let (root, card) = library_with_card("accepted");
    let mut app = start(&root);
    app.wait_for("role:button[gato]")?;
    app.press("role:button[gato]")?;
    app.wait_for("Card source")?;
    app.type_text("label:Card source", "!")?;
    app.wait_for("Unsaved")?;
    app.press("role:button[Save]")?;
    app.wait_for("Saved")?;
    let stored = Db::open(&root.join("typr.sqlite3"))
        .unwrap()
        .snapshot()
        .unwrap();
    let markdown = &stored.cards.iter().find(|c| c.id == card).unwrap().markdown;
    assert!(markdown.starts_with('!'), "{markdown}");
    Ok(())
}

#[test]
fn an_edited_template_field_is_stored_with_the_card() -> Result<(), String> {
    use typr_core::store::Command;
    let _turn = DATA_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let (root, card) = library_with_card("fields");
    let db = Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = db.snapshot().unwrap().decks[0].id;
    db.apply(
        &Command::SetDeckTemplate {
            deck,
            template: Some("<<Word>>\n---\n<<Meaning>>".to_owned()),
        },
        0,
    )
    .unwrap();
    let fields = |meaning: &str| Command::SetFields {
        card,
        fields: vec![
            ("Word".to_owned(), "gato".to_owned()),
            ("Meaning".to_owned(), meaning.to_owned()),
        ],
    };
    db.apply(&fields("cat"), 0).unwrap();
    drop(db);

    let mut app = start(&root);
    app.wait_for("role:button[gato]")?;
    app.press("role:button[gato]")?;
    app.wait_for("Fields")?;
    app.clear("text:cat >> nth=2")?;
    app.type_text("text:Value", "feline")?;
    app.wait_for("Unsaved")?;
    app.press("role:button[Save]")?;
    app.wait_for("Saved")?;
    let stored = Db::open(&root.join("typr.sqlite3"))
        .unwrap()
        .snapshot()
        .unwrap();
    let stored = stored.cards.iter().find(|c| c.id == card).unwrap();
    assert!(
        stored.markdown.contains("feline"),
        "the card text is rendered from the edited field: {}",
        stored.markdown
    );
    Ok(())
}

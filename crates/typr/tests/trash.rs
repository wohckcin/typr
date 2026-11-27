//! The trash screen: trashed cards newest first, Restore, and Empty trash behind a confirmation.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::trash::{trash, trash_rows};
use typr_core::store::{CardSummary, Deck, Snapshot};

fn trashed(id: i64, preview: &str, at: i64) -> CardSummary {
    let mut card = CardSummary::default();
    card.id = id;
    card.deck = 1;
    card.preview = preview.into();
    card.trashed_at = Some(at);
    card
}

fn snapshot(cards: Vec<CardSummary>) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.decks = vec![Deck::new(1, "Flashcards")];
    snapshot.cards = cards;
    snapshot
}

#[test]
fn rows_are_newest_first_and_name_their_deck() {
    let mut live = CardSummary::default();
    live.id = 3;
    live.deck = 1;
    let rows = trash_rows(&snapshot(vec![
        trashed(1, "A", 10),
        trashed(2, "B", 20),
        live,
    ]));
    let previews: Vec<&str> = rows.iter().map(|r| r.preview.as_str()).collect();
    assert_eq!(previews, ["B", "A"], "a live card is not in the trash");
    assert!(rows.iter().all(|r| r.deck == "Flashcards"));
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let shown = signal(snapshot(vec![trashed(1, "A", 10), trashed(2, "B", 20)]));
    let log = signal(String::new());
    view()
        .child(trash(
            shown,
            move |id| {
                log.update(|l| l.push_str(&format!("restored {id};")));
                shown.update(|s| s.cards.retain(|c| c.id != id));
            },
            move || {
                log.update(|l| l.push_str("emptied;"));
                shown.update(|s| s.cards.clear());
            },
        ))
        .child(text(move || format!("log={}", log.get())))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn restore_takes_a_card_out_of_the_list() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:B")?;
    app.press("role:button[Restore] >> nth=0")?;
    app.wait_for("text:/restored 2;/")?;
    app.wait_for_gone("text:B", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("A"));
    Ok(())
}

#[test]
fn emptying_asks_first_and_cancel_keeps_the_cards() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:A")?;
    app.press("Empty trash")?;
    app.wait_for("text:/Destroy 2 cards\\?/")?;
    app.press("Cancel")?;
    app.wait_for("Empty trash")?;
    assert!(app.screen().shows("A") && app.screen().shows("B"));
    assert!(!app.screen().shows("emptied;"));
    Ok(())
}

#[test]
fn confirming_empties_the_trash_and_says_so() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:A")?;
    app.press("Empty trash")?;
    app.press("role:button[Destroy]")?;
    app.wait_for("text:/emptied;/")?;
    app.wait_for("Trash is empty")?;
    app.wait_for("Deleted cards stay here until you empty the trash")?;
    assert!(!app.screen().shows("Empty trash"));
    Ok(())
}

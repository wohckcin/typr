//! The library screen: the sidebar beside the grid of the selected deck, with a search field over
//! the tiles. Cards are ordered pinned first, then by their place in the deck.

use nsh_core::prelude::*;
use typr_core::store::{CardSummary, Snapshot};

use super::grid::{Tile, grid, tile_of};
use super::sidebar::{Sidebar, due_counts, items_from, search_chord, sidebar};
use crate::ui::{input::field, tokens};

/// Which cards a selection shows: every live card for `all`, or the cards of one deck `deck:<id>`.
pub fn cards_in(snapshot: &Snapshot, selection: Option<&str>) -> Vec<CardSummary> {
    let deck: Option<i64> = selection
        .and_then(|key| key.strip_prefix("deck:"))
        .and_then(|id| id.parse().ok());
    snapshot
        .cards
        .iter()
        .filter(|card| !card.archived && card.trashed_at.is_none())
        .filter(|card| deck.is_none_or(|d| card.deck == d))
        .cloned()
        .collect()
}

/// Tiles for `cards` whose preview contains `query` (case-insensitive), pinned cards first.
pub fn tiles_for(cards: &[CardSummary], query: &str, now_ms: i64) -> Vec<Tile> {
    let needle = query.to_lowercase();
    let mut matching: Vec<&CardSummary> = cards
        .iter()
        .filter(|card| needle.is_empty() || card.preview.to_lowercase().contains(&needle))
        .collect();
    matching.sort_by_key(|card| (!card.pinned, card.position));
    matching
        .into_iter()
        .map(|card| tile_of(card, now_ms))
        .collect()
}

/// The library screen over `snapshot`, with the sidebar's selection choosing the cards.
pub fn library(
    snapshot: Snapshot,
    library_open: bool,
    now_ms: i64,
    on_open: impl Fn(i64) + Clone + 'static,
) -> impl IntoElement {
    let due = due_counts(&snapshot, now_ms);
    let snapshot = std::rc::Rc::new(snapshot);
    let state = Sidebar::new(items_from(&snapshot, &due), library_open, search_chord());
    let query = signal(String::new());
    let chosen = signal(0);
    let tiles = computed({
        let snapshot = snapshot.clone();
        move || {
            let selection = state.selected_key();
            let cards = cards_in(&snapshot, selection.as_deref());
            tiles_for(&cards, &query.get(), now_ms)
        }
    });
    view()
        .flex_1()
        .flex_row()
        .bg(tokens::background)
        .child(sidebar(state))
        .child(
            view()
                .flex_1()
                .p_4()
                .gap_3()
                .child(field("Search cards", query))
                .child(grid(tiles, chosen, || {}, on_open)),
        )
}

//! The library's cards: which of them a selection shows, and the tiles the grid draws for them.
//! The screen itself is composed by `shell`, so there is one drawing of it.

use typr_core::inbox::{Inbox, inbox_cards};
use typr_core::store::{CardSummary, Snapshot};

use super::grid::{Tile, tile_of};

/// Which cards a selection shows: the Inbox rows' cards, the cards of one deck `deck:<id>`, or every
/// live card for any other key.
pub fn cards_in(snapshot: &Snapshot, selection: Option<&str>, now_ms: i64) -> Vec<CardSummary> {
    if let Some(which) = selection.and_then(Inbox::of_key) {
        return inbox_cards(snapshot, now_ms, which);
    }
    let deck = selection.and_then(typr_core::edits::deck_of_key);
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

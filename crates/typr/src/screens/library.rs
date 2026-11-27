//! The library's cards: which of them a selection shows, and the tiles the grid draws for them.
//! The screen itself is composed by `shell`, so there is one drawing of it.

pub use typr_core::inbox::cards_in;
use typr_core::store::CardSummary;

use super::grid::{Tile, tile_of};

/// Why the grid is empty, for the selected `key` and the search `query`.
pub fn empty_message(key: Option<&str>, query: &str) -> &'static str {
    if !query.is_empty() {
        return "No cards match your search";
    }
    match key {
        Some("inbox:due") => "Nothing is due. You're done for today",
        Some("inbox:new") => "No new cards to learn",
        Some("inbox") => "No cards are due or new",
        _ => "This deck has no cards yet",
    }
}

/// Whether an empty grid offers New card: not under a search or a study filter, where a new card
/// would not show.
pub fn offers_new_card(key: Option<&str>, query: &str) -> bool {
    query.is_empty() && !matches!(key, Some("inbox:due" | "inbox:new"))
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

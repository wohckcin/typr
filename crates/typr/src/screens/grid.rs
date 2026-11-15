//! The card grid: the cards of the selected deck as tiles in a virtual list, each showing its
//! first side, how many sides it hides, and its tags.
//!
//! The tiles are data; a cell is made once and given other tiles as it is recycled, so it reads its
//! row through a computed keyed by the index.

use nsh_core::prelude::*;

use crate::ui::{status::badge, surface, tokens};

/// One card's tile: what it shows, read from the snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    /// Identifies the card; the list keys its rows on it.
    pub id: i64,
    /// The first side, as its preview text.
    pub preview: SharedString,
    /// Sides beyond the first, which the tile states rather than shows.
    pub hidden_sides: usize,
    /// The card's tags, in the order they were set.
    pub tags: Vec<SharedString>,
}

/// The grid over `tiles`. An empty deck offers the tile that starts a card.
pub fn grid(tiles: Signal<Vec<Tile>>, on_new: impl Fn() + Clone + 'static) -> impl IntoElement {
    let empty_new = on_new.clone();
    view().flex_1().child(dynamic(move || {
        if tiles.with(Vec::is_empty) {
            let on_new = empty_new.clone();
            return view()
                .p_4()
                .child(badge("New card"))
                .child(text("This deck has no cards yet").text_sm().text_color(move || tokens::muted_foreground()))
                .child(button_new(on_new))
                .into_element();
        }
        virtual_list(move || tiles.with(Vec::len), move |at| tile_cell(tiles, at))
            .keys(move || tiles.with(|t| t.iter().map(|t| t.id).collect::<Vec<_>>()))
            .estimated_item_size(px(96.0))
            .flex_1()
            .into_element()
    }))
}

fn button_new(on_new: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::button::button(crate::ui::button::Variant::Default, "New card", || false, on_new)
}

/// A cell of the list. It keeps no state: every binding reads the tile at its current index.
fn tile_cell(tiles: Signal<Vec<Tile>>, at: Position) -> impl IntoElement {
    let tile = computed(move || tiles.with(|t| t.get(at.index()).cloned()));
    let preview = move || tile.with(|t| t.as_ref().map(|t| t.preview.clone()).unwrap_or_default());
    let hidden = move || tile.with(|t| t.as_ref().map_or(0, |t| t.hidden_sides));
    let tag_row = move || {
        let tags = tile.with(|t| t.as_ref().map(|t| t.tags.clone()).unwrap_or_default());
        let mut row = view().flex_row().gap_1();
        for tag in tags {
            row = row.child(badge(tag));
        }
        row
    };
    surface::card(
        view()
            .gap_2()
            .child(text(preview).text_sm().text_color(move || tokens::foreground()))
            .child(dynamic(move || {
                let n = hidden();
                if n == 0 {
                    return view().into_element();
                }
                let label = if n == 1 { "1 hidden side".to_owned() } else { format!("{n} hidden sides") };
                text(label).text_xs().text_color(move || tokens::muted_foreground()).into_element()
            }))
            .child(dynamic(move || tag_row().into_element())),
    )
}

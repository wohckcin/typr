//! The card grid: the cards of the selected deck as tiles in a virtual list, each showing its
//! first side, how many sides it hides, and its tags.
//!
//! The tiles are data; a cell is made once and given other tiles as it is recycled, so it reads its
//! row through a computed keyed by the index.

use nsh_core::prelude::*;

use crate::ui::{prose::prose, status::badge, surface, tokens};

/// How a card's schedule reads on its tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// Never graded: the tile says it is new.
    New,
    /// Graded and due later: the days until it comes back.
    Interval(u32),
    /// Graded and due now.
    Due,
}

/// The width a tile is given: the window less the page's padding, shared across its columns.
fn tile_width() -> f32 {
    let columns = tokens::grid_columns(window().width.0) as f32;
    (window().width.0 - 32.0) / columns
}

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
    /// Where the card's schedule stands.
    pub schedule: Schedule,
    /// Whether the card is pinned to the top of its deck.
    pub pinned: bool,
}

const DAY_MS: i64 = 86_400_000;

/// The schedule state of a card at `now_ms`.
pub fn schedule_of(reviews: u32, due_at: Option<i64>, now_ms: i64) -> Schedule {
    match due_at {
        _ if reviews == 0 || due_at.is_none() => Schedule::New,
        Some(due) if due <= now_ms => Schedule::Due,
        Some(due) => Schedule::Interval(((due - now_ms) / DAY_MS) as u32),
        None => Schedule::New,
    }
}

/// The first side of a card as a tile shows it. A tile shows the question, so every hidden span is
/// masked and labelled by its group; a split side loses its trailing line break, which would read as
/// an empty line. Masking only the first group would leave the other answers readable in the grid.
pub fn first_side(markdown: &str) -> String {
    let shape = typr_core::hidden::shape(markdown);
    match &shape {
        typr_core::hidden::Shape::Hidden { .. } => typr_core::hidden::masked_all(markdown, &shape),
        typr_core::hidden::Shape::Sides(_) => typr_core::sides::split(markdown)
            .first()
            .map(|side| side.trim_end().to_string())
            .unwrap_or_default(),
    }
}

/// The tile a card draws, from its snapshot row.
pub fn tile_of(card: &typr_core::store::CardSummary, now_ms: i64) -> Tile {
    Tile {
        id: card.id,
        preview: first_side(&card.markdown).into(),
        hidden_sides: (card.sides as usize).saturating_sub(1),
        tags: card.tags.iter().map(|t| SharedString::from(t.clone())).collect(),
        schedule: schedule_of(card.reviews, card.due_at, now_ms),
        pinned: card.pinned,
    }
}

/// The grid over `tiles`. An empty deck offers the tile that starts a card.
pub fn grid(tiles: Memo<Vec<Tile>>, on_new: impl Fn() + Clone + 'static) -> impl IntoElement {
    let empty_new = on_new.clone();
    view().flex_1().child(dynamic(move || {
        if tiles.with(Vec::is_empty) {
            let on_new = empty_new.clone();
            let chord = typr_core::keymap::chord_of(typr_core::keymap::Surface::Library, typr_core::keymap::Command::NewCard)
                .map(|c| c.to_string())
                .unwrap_or_default();
            return view()
                .p_4()
                .child(badge("New card"))
                .child(text(chord).text_xs().text_color(move || tokens::muted_foreground()))
                .child(text("This deck has no cards yet").text_sm().text_color(move || tokens::muted_foreground()))
                .child(button_new(on_new))
                .into_element();
        }
        virtual_list(move || tiles.with(Vec::len), move |at| tile_cell(tiles, at))
            .columns(move || tokens::grid_columns(window().width.0))
            .keys(move || tiles.with(|t| t.iter().map(|t| t.id).collect::<Vec<_>>()))
            .estimated_item_size(tokens::tile_estimate())
            .flex_1()
            .into_element()
    }))
}

fn button_new(on_new: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::button::button(crate::ui::button::Variant::Default, "New card", || false, on_new)
}

/// A cell of the list. It keeps no state: every binding reads the tile at its current index.
fn tile_cell(tiles: Memo<Vec<Tile>>, at: Position) -> impl IntoElement {
    let tile = computed(move || tiles.with(|t| t.get(at.index()).cloned()));
    let preview = move || tile.with(|t| t.as_ref().map(|t| t.preview.clone()).unwrap_or_default());
    let hidden = move || tile.with(|t| t.as_ref().map_or(0, |t| t.hidden_sides));
    let schedule = move || {
        let label = tile.with(|t| match t.as_ref().map(|t| t.schedule) {
            Some(Schedule::New) | None => "New".to_owned(),
            Some(Schedule::Interval(days)) => format!("{days} days"),
            Some(Schedule::Due) => "Due".to_owned(),
        });
        label
    };
    let tag_row = move || {
        let tags = tile.with(|t| t.as_ref().map(|t| t.tags.clone()).unwrap_or_default());
        let width = tile_width();
        let names: Vec<&str> = tags.iter().map(|t| t.as_ref()).collect();
        let shown = tokens::tags_that_fit(width, &names);
        let more = tags.len() - shown;
        let mut row = view().flex_row().gap_1();
        for tag in tags.into_iter().take(shown) {
            row = row.child(badge(tag));
        }
        if more > 0 {
            row = row.child(badge(format!("{more} more")));
        }
        row
    };
    surface::card(
        view()
            .gap_2()
            .child(prose(preview))
            .child(dynamic(move || {
                if !tile.with(|t| t.as_ref().is_some_and(|t| t.pinned)) {
                    return view().into_element();
                }
                badge("Pinned").into_element()
            }))
            .child(dynamic(move || {
                let n = hidden();
                if n == 0 {
                    return view().into_element();
                }
                let label = if n == 1 { "1 hidden side".to_owned() } else { format!("{n} hidden sides") };
                text(label).text_xs().text_color(move || tokens::muted_foreground()).into_element()
            }))
            .child(text(schedule).text_xs().text_color(move || tokens::muted_foreground()))
            .child(dynamic(move || tag_row().into_element())),
    )
}

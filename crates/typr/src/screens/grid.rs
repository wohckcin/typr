//! The card grid: the cards of the selected deck as tiles in a virtual list, each showing its
//! first side, how many sides it hides, and its tags.
//!
//! The tiles are data; a cell is made once and given other tiles as it is recycled, so it reads its
//! row through a computed keyed by the index.

use nsh_core::prelude::*;

pub use typr_core::schedule::{Schedule, schedule_of};
use typr_core::store::Id;

use crate::ui::{prose::prose, status::badge, surface, tokens};

/// The width a tile is given: the window less the page's padding, shared across its columns.
/// The width the grid has to work with: the window less the sidebar and the page's padding. The
/// tile count and each tile's width come from this, so a narrow window shows one full tile rather
/// than two squeezed ones.
fn content_width() -> f32 {
    (window().width.0 - tokens::sidebar_width().0 - 32.0).max(0.0)
}

fn tile_width() -> f32 {
    let columns = tokens::grid_columns(content_width()) as f32;
    (content_width() - (columns - 1.0) * tokens::GRID_GAP) / columns
}

/// One card's tile: what it shows, read from the snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
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

impl Tile {
    /// A new tile for `id` showing `preview`, with no hidden sides, tags, or pin.
    pub fn new(id: i64, preview: impl Into<SharedString>) -> Self {
        Self {
            id,
            preview: preview.into(),
            ..Self::default()
        }
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
    // A diagram has no text: its tile says what the picture shows.
    let preview = match card.kind {
        typr_core::store::CardKind::Diagram => card.caption.clone().unwrap_or_default(),
        _ => first_side(&card.markdown),
    };
    Tile {
        id: card.id,
        preview: preview.into(),
        hidden_sides: (card.sides as usize).saturating_sub(1),
        tags: card
            .tags
            .iter()
            .map(|t| SharedString::from(t.clone()))
            .collect(),
        schedule: schedule_of(card.reviews, card.due_at, now_ms),
        pinned: card.pinned,
    }
}

/// The grid over `tiles`. An empty grid says `empty`, and offers New card when `offer_new`.
pub fn grid(
    tiles: Memo<Vec<Tile>>,
    chosen: Signal<Option<Id>>,
    empty: impl Fn() -> &'static str + 'static,
    offer_new: impl Fn() -> bool + 'static,
    on_new: impl Fn() + Clone + 'static,
    on_open: impl Fn(i64) + Clone + 'static,
) -> impl IntoElement {
    let empty_new = on_new.clone();
    let open = on_open.clone();
    view().flex_1().child(dynamic(move || {
        if tiles.with(Vec::is_empty) {
            let mut page = view()
                .p_4()
                .child(text(empty()).text_sm().text_color(tokens::muted_foreground));
            if offer_new() {
                page = page.child(button_new(empty_new.clone()));
            }
            return page.into_element();
        }
        let open = open.clone();
        virtual_list(
            move || tiles.with(Vec::len),
            move |at| tile_cell(tiles, at, chosen, open.clone()),
        )
        .columns(move || tokens::grid_columns(content_width()))
        .row_gap(tokens::grid_gap())
        .column_gap(tokens::grid_gap())
        .keys(move || tiles.with(|t| t.iter().map(|t| t.id).collect::<Vec<_>>()))
        .estimated_item_size(tokens::tile_estimate())
        .flex_1()
        .into_element()
    }))
}

fn button_new(on_new: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::button::button()
        .label("New card")
        .disabled(|| false)
        .on_press(on_new)
}

/// A cell of the list. It keeps no state: every binding reads the tile at its current index.
fn tile_cell(
    tiles: Memo<Vec<Tile>>,
    at: Position,
    chosen: Signal<Option<Id>>,
    on_open: impl Fn(i64) + Clone + 'static,
) -> impl IntoElement {
    let tile = computed(move || tiles.with(|t| t.get(at.index()).cloned()));
    let preview = move || tile.with(|t| t.as_ref().map(|t| t.preview.clone()).unwrap_or_default());
    let hidden = move || tile.with(|t| t.as_ref().map_or(0, |t| t.hidden_sides));
    let schedule = move || {
        tile.with(|t| match t.as_ref().map(|t| t.schedule) {
            Some(Schedule::New) | None => "New".to_owned(),
            Some(Schedule::Interval(0)) => "Under a day".to_owned(),
            Some(Schedule::Interval(1)) => "1 day".to_owned(),
            Some(Schedule::Interval(days)) => format!("{days} days"),
            Some(Schedule::Due) => "Due".to_owned(),
        })
    };
    let tag_row = move || {
        let tags = tile.with(|t| t.as_ref().map(|t| t.tags.clone()).unwrap_or_default());
        let width = tile_width();
        let names: Vec<&str> = tags.iter().map(|t| t.as_ref()).collect();
        let shown = tokens::tags_that_fit(width, &names);
        let more = tags.len() - shown;
        let mut row = view().flex_row().gap_1();
        for tag in tags.into_iter().take(shown) {
            row = row.child(badge().label(tag));
        }
        if more > 0 {
            row = row.child(badge().label(format!("{more} more")));
        }
        row
    };
    let opened = move || tile.with(|t| t.as_ref().map(|t| t.id).unwrap_or_default());
    let press = {
        let on_open = on_open.clone();
        move || on_open(opened())
    };
    let named = move || -> SharedString { typr_core::markdown_ext::plain(&preview()).into() };
    // The keyboard's card wears a ring, by identity so a filter or a reorder cannot move it to a
    // different card at the same position.
    let ring = move || {
        let chosen_id = chosen.get();
        let is_chosen = tile.with(|t| t.as_ref().is_some_and(|t| chosen_id == Some(t.id)));
        if is_chosen {
            tokens::selection()
        } else {
            tokens::background()
        }
    };
    crate::ui::button::tile()
        .label(named)
        .on_press(press)
        .child(
            view()
                .rounded(tokens::radius)
                .border_1()
                .border_color(ring)
                .child(
                    surface::surface().p_3().child(
                        view()
                            .gap_2()
                            .child(prose().source(preview))
                            .child(dynamic(move || {
                                if !tile.with(|t| t.as_ref().is_some_and(|t| t.pinned)) {
                                    return view().into_element();
                                }
                                badge().label("Pinned").into_element()
                            }))
                            .child(dynamic(move || {
                                let n = hidden();
                                if n == 0 {
                                    return view().into_element();
                                }
                                let label = if n == 1 {
                                    "1 hidden side".to_owned()
                                } else {
                                    format!("{n} hidden sides")
                                };
                                text(label)
                                    .text_xs()
                                    .text_color(tokens::muted_foreground)
                                    .into_element()
                            }))
                            .child(
                                text(schedule)
                                    .text_xs()
                                    .text_color(tokens::muted_foreground),
                            )
                            .child(dynamic(move || tag_row().into_element())),
                    ),
                ),
        )
}

//! The library screen: decks and tags on the left, a virtualised card list on the right.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rune::tok::{self, type_};
use rune::{El, GrowthSpec, ScrollSpec, TextEdit, col, row, text, text_input};

use crate::store::{CardMode, CardSummary, Deck, Id, Snapshot};

/// How tall one card row is, which the list needs before it has laid anything out.
/// The height of one card row, in pixels.
pub const ROW_HEIGHT_PX: u32 = 44;
pub const ROW_HEIGHT: f32 = ROW_HEIGHT_PX as f32;

/// The app state a library screen draws from. Rune binds every callback to the app's own state
/// type, so the screen cannot name one implementation.
pub trait Host: 'static {
    fn library(&self) -> &Library;
    fn library_mut(&mut self) -> &mut Library;
}

impl Host for Library {
    fn library(&self) -> &Library {
        self
    }

    fn library_mut(&mut self) -> &mut Library {
        self
    }
}

/// What the library screen shows and what the user is filtering it by.
pub struct Library {
    /// The published library this list is a view of.
    snapshot: Arc<Snapshot>,
    pub mode: CardMode,
    pub selected_deck: Option<Id>,
    pub selected_card: Option<Id>,
    pub query: TextEdit,
    pub tag: Option<String>,
    /// The rows the filters keep, rebuilt whenever a filter changes.
    /// The cards the current deck, tag, search and mode select, in list order.
    visible: Vec<CardSummary>,
    /// Every index the list has built. A virtualised list builds only what is on screen, so a
    /// large library must leave this far below the row count.
    pub built_rows: AtomicUsize,
}

impl Default for Library {
    fn default() -> Self {
        Library {
            snapshot: Arc::default(),
            mode: CardMode::Active,
            selected_deck: None,
            selected_card: None,
            query: TextEdit::default(),
            tag: None,
            visible: Vec::new(),
            built_rows: AtomicUsize::new(0),
        }
    }
}

impl Library {
    /// Replace the library the screen draws, and rebuild the rows the filters keep.
    /// The library this list shows.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// The cards on screen, in the order they are drawn.
    pub fn cards(&self) -> &[CardSummary] {
        &self.visible
    }

    /// Take a newly published library and rebuild the list from it. The only way in, so the two
    /// can never disagree.
    pub fn set_snapshot(&mut self, snapshot: Arc<Snapshot>) {
        self.snapshot = snapshot;
        self.refresh();
    }

    /// Recompute the visible rows from the current filters.
    fn refresh(&mut self) {
        self.visible = self
            .snapshot
            .cards_matching(
                &self.query.text,
                self.tag.as_deref(),
                self.selected_deck,
                self.mode,
            )
            .into_iter()
            .cloned()
            .collect();
    }

    /// A snapshot with `count` cards in one deck, for scenes and for the virtualised-list check.
    pub fn seeded(count: usize) -> Library {
        let deck = Deck {
            id: 1,
            parent: None,
            name: "Seeded".to_string(),
            archived: false,
        };
        let mut library = Library {
            snapshot: Arc::new(Snapshot {
                decks: vec![deck],
                ..Snapshot::default()
            }),
            ..Library::default()
        };
        Arc::make_mut(&mut library.snapshot).cards = (1..=count as Id)
            .map(|id| CardSummary {
                id,
                deck: 1,
                sides: 2,
                tags: Vec::new(),
                archived: false,
                trashed_at: None,
                updated_at: 0,
                preview: format!("Card {id}"),
                markdown: format!("Card {id}\n---\nCard {id}"),
                search_text: format!("card {id}"),
            })
            .collect();
        library.refresh();
        library
    }
}

fn deck_row<S: Host>(deck: &Deck, selected: bool) -> El {
    let t = tok::theme(true);
    let id = deck.id;
    row()
        .key(format!("deck-{}", deck.id))
        .h(30.0)
        .px(10.0)
        .radius(6.0)
        .center_y()
        .fill(if selected {
            tok::accent(0, true).alpha(0.85)
        } else {
            t.control_off.alpha(0.0)
        })
        .child(
            text(deck.name.clone())
                .font(type_::BODY)
                .color(if selected { rune::Color::WHITE } else { t.text }),
        )
        .on_press(move |s: &mut S| {
            let library = s.library_mut();
            library.selected_deck = Some(id);
            library.refresh();
        })
}

fn deck_sidebar<S: Host>(s: &S) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let mut sidebar = col()
        .key("decks")
        .w(200.0)
        .h_full()
        .padding(8.0)
        .gap(2.0)
        .fill(t.card);
    let all = row()
        .key("deck-all")
        .h(30.0)
        .px(10.0)
        .radius(6.0)
        .center_y()
        .fill(if library.selected_deck.is_none() {
            tok::accent(0, true)
        } else {
            t.control_off.alpha(0.0)
        })
        .child(text("All cards").font(type_::BODY).color(t.text))
        .on_press(|s: &mut S| {
            let library = s.library_mut();
            library.selected_deck = None;
            library.refresh();
        });
    sidebar = sidebar.child(all);
    for deck in &library.snapshot.decks {
        sidebar = sidebar.child(deck_row::<S>(deck, library.selected_deck == Some(deck.id)));
    }
    sidebar
}

fn filter_bar<S: Host>(s: &S) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let search = text_input(&library.query, type_::BODY.lh(20.0))
        .key("search")
        .single_line()
        .grow(1.0)
        .placeholder("Search cards", t.text_secondary)
        .on_edit(|s: &mut S| &mut s.library_mut().query)
        .on_input_change(|s: &mut S, _| s.library_mut().refresh());
    row()
        .key("filters")
        .h(44.0)
        .px(12.0)
        .center_y()
        .gap(8.0)
        .child(search)
        .child(
            text(
                match (library.visible.len(), library.snapshot.cards.len()) {
                    (shown, total) if shown == total => format!("{total} cards"),
                    (shown, total) => format!("{shown} of {total}"),
                },
            )
            .font(type_::LABEL)
            .color(t.text_secondary),
        )
}

/// One row of the card list.
fn card_row<S: Host>(card: &CardSummary, selected: bool) -> El {
    let t = tok::theme(true);
    let id = card.id;
    let mut tags = String::new();
    for tag in &card.tags {
        if !tags.is_empty() {
            tags.push(' ');
        }
        tags.push('#');
        tags.push_str(tag);
    }
    let note = if card.sides < 2 { "note" } else { "" };
    row()
        .key(format!("card-{}", card.id))
        .h(ROW_HEIGHT)
        .px(12.0)
        .center_y()
        .gap(8.0)
        .fill(if selected {
            t.control_off
        } else {
            t.control_off.alpha(0.0)
        })
        .child(
            text(if card.preview.is_empty() {
                "(empty)".to_string()
            } else {
                card.preview.clone()
            })
            .font(type_::BODY)
            .grow(1.0)
            .color(t.text),
        )
        .child(text(tags).font(type_::LABEL).color(t.text_secondary))
        .child(text(note).font(type_::LABEL).color(t.text_secondary))
        .on_press(move |s: &mut S| s.library_mut().selected_card = Some(id))
}

/// The library screen.
pub fn view<S: Host>(s: &S) -> El {
    let library = s.library();
    let list = col()
        .key("cards")
        .grow(1.0)
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("rows")
                .growth(GrowthSpec::default())
                .virtual_rows(
                    library.visible.len(),
                    ROW_HEIGHT,
                    |s: &S, index| -> Cow<'_, str> {
                        format!("card-{}", s.library().visible[index].id).into()
                    },
                    |s: &S, index| {
                        let library = s.library();
                        library.built_rows.fetch_add(1, Ordering::Relaxed);
                        let card = &library.visible[index];
                        card_row::<S>(card, library.selected_card == Some(card.id))
                    },
                ),
        );
    row()
        .w_full()
        .h_full()
        .fill(tok::theme(true).bg)
        .child(deck_sidebar::<S>(s))
        .child(
            col()
                .grow(1.0)
                .h_full()
                .child(filter_bar::<S>(s))
                .child(list),
        )
}

#[cfg(test)]
mod tests;

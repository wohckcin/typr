//! The library screen: decks and tags on the left, a virtualised card list on the right.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rune::tok::{self, type_};
use rune::{El, GrowthSpec, ScrollSpec, TextEdit, col, row, text, text_input};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, root};

use typr_core::store::{CardMode, CardSummary, Deck, Id, Snapshot};

/// Kept for the lifetime of the app: a Kit rebuilds its palette and type ramp.
static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

/// How tall one card row is, which the list needs before it has laid anything out.
/// The height of one card row, in pixels.
pub const ROW_HEIGHT_PX: u32 = 44;
pub const ROW_HEIGHT: f32 = ROW_HEIGHT_PX as f32;

/// The app state a library screen draws from. Rune binds every callback to the app's own state
/// type, so the screen cannot name one implementation.
pub trait Host: 'static {
    fn library(&self) -> &Library;
    fn library_mut(&mut self) -> &mut Library;

    /// Switch the list to another view. The default needs no store: it only changes what is drawn.
    fn set_mode(&mut self, mode: CardMode) {
        self.library_mut().set_mode(mode);
    }

    /// Take a card out of the library without deleting it.
    fn archive_card(&mut self, _card: Id) {}

    /// Take a whole deck, and everything under it, out of the library.
    fn archive_deck(&mut self, _deck: Id) {}

    /// Archive a deck, asking first when it would take cards with it.
    fn request_archive_deck(&mut self, deck: Id) {
        if self.library().holds_cards(deck) {
            self.library_mut().confirming_archive = Some(deck);
        } else {
            self.archive_deck(deck);
        }
    }

    /// Move a card to the trash, where it waits until the user empties it.
    fn trash_card(&mut self, _card: Id) {}

    /// Open a card for editing.
    fn open_card(&mut self, _card: Id) {}

    /// Open an editor on a card that does not exist yet.
    fn new_card(&mut self) {}

    /// Move a card into a deck, keeping whatever schedule it has.
    fn move_card(&mut self, _card: Id, _deck: Id) {}

    /// Add a deck under the selected one, or at the top when none is selected.
    fn new_deck(&mut self) {}

    /// Rename a deck. A host with no store cannot, and says so by doing nothing.
    fn rename_deck(&mut self, _deck: Id, _name: &str) {}

    /// Move a deck under another, or to the top of the list when `parent` is `None`.
    fn move_deck(&mut self, _deck: Id, _parent: Option<Id>) {}

    /// Close the name field and send the name. False when no deck was being renamed.
    fn finish_rename(&mut self) -> bool {
        match self.library_mut().take_rename() {
            Some((deck, name)) => {
                self.rename_deck(deck, &name);
                true
            }
            None => false,
        }
    }

    /// Open the name field on the selected deck.
    fn begin_rename(&mut self) {
        if let Some(deck) = self.library().selected_deck {
            self.library_mut().start_rename(deck);
        }
    }

    /// Bring a card out of the archive or the trash. A host with no store cannot, and says so by
    /// doing nothing.
    fn restore_card(&mut self, _card: Id) {}

    /// Destroy everything in the trash. A host with no store leaves it alone.
    fn empty_trash(&mut self) {}
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
    /// Whether the empty-trash question is on screen. Emptying destroys history, so it asks.
    pub confirming_empty: bool,
    /// The deck whose name is being edited, if any.
    pub renaming: Option<Id>,
    /// The name being typed, kept apart from the deck so a cancel costs nothing.
    pub rename_buffer: TextEdit,
    /// The deck waiting for a destination, while the sidebar offers itself as one.
    pub moving: Option<Id>,
    /// The deck waiting for an answer to "archive this?", when it holds cards.
    pub confirming_archive: Option<Id>,
    /// The card waiting for a deck, while the sidebar offers itself as one.
    pub moving_card: Option<Id>,
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
            confirming_empty: false,
            renaming: None,
            rename_buffer: TextEdit::default(),
            moving: None,
            confirming_archive: None,
            moving_card: None,
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

    /// Whether archiving this deck would hide any card, which is what makes it worth asking about.
    pub fn holds_cards(&self, deck: Id) -> bool {
        let decks = self.snapshot.subtree(Some(deck));
        self.snapshot
            .cards
            .iter()
            .any(|card| !card.archived && card.trashed_at.is_none() && decks.contains(&card.deck))
    }

    /// Choose which deck the list shows, redrawing it for that deck.
    pub fn select_deck(&mut self, deck: Option<Id>) {
        self.selected_deck = deck;
        self.refresh();
    }

    /// Open the name field on a deck, holding what the deck is called now.
    pub fn start_rename(&mut self, deck: Id) {
        if let Some(found) = self.snapshot.decks.iter().find(|own| own.id == deck) {
            self.rename_buffer.replace_text(found.name.clone());
            self.renaming = Some(deck);
        }
    }

    /// The name field, for the input that edits it.
    pub fn rename_field(&mut self) -> &mut TextEdit {
        &mut self.rename_buffer
    }

    /// Finish the edit, handing back what was typed. Taking it closes the field, so a rename that
    /// the store refuses does not leave the field open behind the failure.
    pub fn take_rename(&mut self) -> Option<(Id, String)> {
        let deck = self.renaming?;
        Some((deck, self.rename_buffer.text.clone()))
    }

    /// Show another slice of the library: active cards, the archive, or the trash.
    pub fn set_mode(&mut self, mode: CardMode) {
        if self.mode != mode {
            self.mode = mode;
            self.refresh();
        }
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

    /// A library holding active, archived and trashed cards, so the views have something to show.
    pub fn seeded_mixed() -> Library {
        let mut library = Library::seeded(6);
        {
            let snapshot = Arc::make_mut(&mut library.snapshot);
            snapshot.cards[1].archived = true;
            snapshot.cards[2].archived = true;
            snapshot.cards[3].trashed_at = Some(1);
            snapshot.cards[4].trashed_at = Some(1);
        }
        library.refresh();
        library
    }
}

/// One view switch: active cards, the archive, or the trash.
fn mode_tab<S: Host>(label: &'static str, mode: CardMode, current: CardMode) -> El {
    let t = tok::theme(true);
    let selected = mode == current;
    row()
        .key(match mode {
            CardMode::Active => "mode-active",
            CardMode::Archived => "mode-archived",
            CardMode::Trash => "mode-trash",
        })
        .h(28.0)
        .px(10.0)
        .radius(6.0)
        .center_y()
        .fill(if selected {
            t.control_off
        } else {
            t.control_off.alpha(0.0)
        })
        .child(text(label).font(type_::LABEL).color(if selected {
            t.text
        } else {
            t.text_secondary
        }))
        .on_press(move |s: &mut S| s.set_mode(mode))
}

/// The button that asks before the trash is destroyed.
fn empty_trash_button<S: Host>() -> El {
    Button::new("Empty trash")
        .variant(ButtonVariant::Ghost)
        .on_press(|s: &mut S| s.library_mut().confirming_empty = true)
        .build(&KIT)
        .key("empty-trash")
}

fn deck_row<S: Host>(deck: &Deck, selected: bool, renaming: bool, moving: bool) -> El {
    let t = tok::theme(true);
    let id = deck.id;
    if renaming {
        return row()
            .key(format!("deck-{}", deck.id))
            .h(30.0)
            .px(10.0)
            .center_y()
            .fill(t.control_off)
            .child(
                text_input(&TextEdit::default(), type_::LABEL)
                    .key("deck-name")
                    .single_line()
                    .grow(1.0)
                    .on_edit(|s: &mut S| s.library_mut().rename_field())
                    .on_submit(|s: &mut S, _| {
                        s.finish_rename();
                    }),
            )
            .on_press(move |s: &mut S| s.library_mut().start_rename(id));
    }
    row()
        .key(format!("deck-{}", deck.id))
        .h(30.0)
        .px(10.0)
        .radius(6.0)
        .center_y()
        .fill(if moving {
            tok::accent(1, true).alpha(0.35)
        } else if selected {
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
            // While a deck is being moved, every deck is a possible new parent; while a card is,
            // every deck is a possible new home.
            if let Some(card) = s.library().moving_card {
                s.library_mut().moving_card = None;
                s.move_card(card, id);
                return;
            }
            if let Some(moving) = s.library().moving {
                s.library_mut().moving = None;
                if moving != id {
                    s.move_deck(moving, Some(id));
                }
                return;
            }
            s.library_mut().select_deck(Some(id));
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
            if s.library().moving_card.is_some() {
                s.library_mut().moving_card = None;
                return;
            }
            if let Some(moving) = s.library().moving {
                s.library_mut().moving = None;
                s.move_deck(moving, None);
                return;
            }
            s.library_mut().select_deck(None);
        });
    sidebar = sidebar.child(all);
    let renaming = library.renaming;
    for deck in &library.snapshot.decks {
        sidebar = sidebar.child(deck_row::<S>(
            deck,
            library.selected_deck == Some(deck.id),
            renaming == Some(deck.id),
            library.moving.is_some() || library.moving_card.is_some(),
        ));
    }
    sidebar = sidebar.child(
        Button::new("New deck")
            .variant(ButtonVariant::Ghost)
            .on_press(|s: &mut S| s.new_deck())
            .build(&KIT)
            .key("new-deck"),
    );
    if library.selected_deck.is_some() {
        sidebar = sidebar
            .child(
                Button::new("Rename")
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut S| s.begin_rename())
                    .build(&KIT)
                    .key("rename-deck"),
            )
            .child(
                Button::new("Move")
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut S| {
                        let selected = s.library().selected_deck;
                        s.library_mut().moving = selected;
                    })
                    .build(&KIT)
                    .key("move-deck"),
            );
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
    let mut bar = row()
        .key("filters")
        .h(44.0)
        .px(12.0)
        .center_y()
        .gap(8.0)
        .child(mode_tab::<S>("Active", CardMode::Active, library.mode))
        .child(mode_tab::<S>("Archived", CardMode::Archived, library.mode))
        .child(mode_tab::<S>("Trash", CardMode::Trash, library.mode))
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
        );
    if library.mode == CardMode::Trash {
        bar = bar.child(empty_trash_button::<S>());
    }
    bar = bar.child(
        Button::new("New card")
            .variant(ButtonVariant::Submit)
            .on_press(|s: &mut S| s.new_card())
            .build(&KIT)
            .key("new-card"),
    );
    if let Some(deck) = library.selected_deck {
        bar = bar.child(
            Button::new("Archive deck")
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.request_archive_deck(deck))
                .build(&KIT)
                .key("archive-deck"),
        );
    }
    bar
}

/// One row of the card list.
fn card_row<S: Host>(card: &CardSummary, selected: bool, mode: CardMode) -> El {
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
    let mut line = row()
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
        .child(text(note).font(type_::LABEL).color(t.text_secondary));
    if mode == CardMode::Active {
        line = line
            .child(
                Button::new("Move")
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| s.library_mut().moving_card = Some(id))
                    .build(&KIT)
                    .key(format!("move-card-{id}")),
            )
            .child(
                Button::new("Edit")
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| s.open_card(id))
                    .build(&KIT)
                    .key(format!("edit-{id}")),
            )
            .child(
                Button::new("Archive")
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| s.archive_card(id))
                    .build(&KIT)
                    .key(format!("archive-{id}")),
            )
            .child(
                Button::new("Delete")
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| s.trash_card(id))
                    .build(&KIT)
                    .key(format!("delete-{id}")),
            );
    } else {
        line = line.child(
            Button::new("Restore")
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.restore_card(id))
                .build(&KIT)
                .key(format!("restore-{id}")),
        );
    }
    line.on_press(move |s: &mut S| s.library_mut().selected_card = Some(id))
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
                        card_row::<S>(card, library.selected_card == Some(card.id), library.mode)
                    },
                ),
        );
    let body = row()
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
        );
    let mut layer = Vec::new();
    confirmation::<S>(s, &mut layer);
    deck_confirmation::<S>(s, &mut layer);
    root(body, layer)
}

/// The question shown before a deck that holds cards would disappear with them.
fn deck_confirmation<S: Host>(s: &S, layer: &mut Vec<El>) {
    let deck = s.library().confirming_archive;
    let buttons = row()
        .key("archive-buttons")
        .w_full()
        .padding(16.0)
        .gap(8.0)
        .justify_end()
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.library_mut().confirming_archive = None)
                .build(&KIT)
                .key("cancel-archive-deck"),
        )
        .child(
            Button::new("Archive deck")
                .variant(ButtonVariant::Submit)
                .on_press(|s: &mut S| {
                    if let Some(deck) = s.library_mut().confirming_archive.take() {
                        s.archive_deck(deck);
                    }
                })
                .build(&KIT)
                .key("confirm-archive-deck"),
        );
    Dialog::new("archive-deck", "Archive this deck?")
        .description("Its cards leave the library, and their schedules stop coming up.")
        .fit()
        .open(deck.is_some())
        .on_close(|s: &mut S| s.library_mut().confirming_archive = None)
        .child(buttons)
        .build(&KIT, layer);
}

/// The question shown before the trash is destroyed, because its history goes with it.
fn confirmation<S: Host>(s: &S, layer: &mut Vec<El>) {
    let buttons = row()
        .key("empty-buttons")
        .w_full()
        .padding(16.0)
        .gap(8.0)
        .justify_end()
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.library_mut().confirming_empty = false)
                .build(&KIT)
                .key("cancel-empty"),
        )
        .child(
            Button::new("Empty trash")
                .variant(ButtonVariant::Submit)
                .on_press(|s: &mut S| {
                    s.library_mut().confirming_empty = false;
                    s.empty_trash();
                })
                .build(&KIT)
                .key("confirm-empty"),
        );
    Dialog::new("empty-trash", "Empty the trash?")
        .description("Every card in the trash and its review history is destroyed.")
        .fit()
        .open(s.library().confirming_empty)
        .on_close(|s: &mut S| s.library_mut().confirming_empty = false)
        .child(buttons)
        .build(&KIT, layer);
}

#[cfg(test)]
mod tests;

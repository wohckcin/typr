//! The library screen: decks and tags on the left, a virtualised card list on the right.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rune::tok::{self, type_};
use rune::{
    El, FileDialogOptions, GrowthSpec, ScrollSpec, TextEdit, Tx, col, row, text, text_input,
};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, Markdown, MarkdownStyle, root};

use typr_core::keymap::{Command, Surface};
use typr_core::library_view::ViewMode;

use typr_core::store::{
    CardMode, CardSummary, Deck, Id, SavedView, Snapshot, Sort, SortDir, ViewSpec,
};

/// Kept for the lifetime of the app: a Kit rebuilds its palette and type ramp.
static KIT: std::sync::LazyLock<Kit> = std::sync::LazyLock::new(|| Kit::neutral(true));

/// How tall one card row is, which the list needs before it has laid anything out.
/// The height of one card row, in pixels.
pub const ROW_HEIGHT_PX: u32 = 44;
pub const ROW_HEIGHT: f32 = ROW_HEIGHT_PX as f32;

/// The space between two tiles, and between two lines of them.
pub const GAP: f32 = 16.0;

/// The width below which a second column stops being worth having. It is the width at which a
/// column wraps, not the width any tile takes: a tile is always exactly its share of the line.
pub const WRAP_BELOW: f32 = 233.0;

/// The height a tile is drawn at before it has been laid out, and the height every tile is drawn
/// at, so a line's tiles match. It is the tallest measured anatomy: name, body, hidden-side strip,
/// tags and footer.
pub const TILE_HEIGHT: f32 = 120.0;

/// The width the grid assumes before it has been laid out: a three-column window.
pub const FALLBACK_GRID_WIDTH: f32 = 767.0;

/// How wide a notebook card reads, and how wide a column view's card reads.
pub const NOTEBOOK_WIDTH: f32 = 760.0;
pub const COLUMN_WIDTH: f32 = 720.0;

/// The tallest a tile may grow, so one long card cannot swallow the deck.
pub const TILE_MAX_HEIGHT: f32 = 160.0;

/// How many tile columns fit across `width`: the width is divided until the next column would be
/// narrower than `WRAP_BELOW`, and there is always at least one.
pub fn columns_for(width: f32) -> usize {
    if !width.is_finite() || width < WRAP_BELOW {
        return 1;
    }
    // Counted in points rather than divided and floored, so nothing is cast from a float.
    let mut columns = 1;
    while (columns as f32 + 1.0) * (WRAP_BELOW + GAP) - GAP <= width {
        columns += 1;
    }
    columns
}

/// How many lines of tiles `cards` takes at `columns` across.
pub fn tile_rows(cards: usize, columns: usize) -> usize {
    cards.div_ceil(columns.max(1))
}

/// The width one tile takes on a line of `columns` filling `width`.
pub fn tile_width(width: f32, columns: usize) -> f32 {
    let columns = columns.max(1);
    ((width - GAP * (columns - 1) as f32) / columns as f32).max(1.0)
}

/// The app state a library screen draws from. Rune binds every callback to the app's own state
/// type, so the screen cannot name one implementation.
pub trait Host: 'static {
    fn library(&self) -> &Library;

    /// Open the settings screen from the library.
    fn open_settings(&mut self) {}
    fn library_mut(&mut self) -> &mut Library;

    /// Whether the window has room for a keycap beside a control's label.
    fn keycaps(&self) -> bool {
        false
    }

    /// Switch the list to another view. The default needs no store: it only changes what is drawn.
    fn set_mode(&mut self, mode: CardMode) {
        self.library_mut().set_mode(mode);
    }

    /// Take a card out of the library without deleting it.
    fn archive_card(&mut self, _card: Id) {}

    /// Take a whole deck, and everything under it, out of the library.
    fn archive_deck(&mut self, _deck: Id) {}

    /// Bring a deck, and everything under it, back into the library.
    fn unarchive_deck(&mut self, _deck: Id) {}

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

    /// Put a card at `position` in its deck, which is what the notebook shows.
    fn reorder_card(&mut self, _card: Id, _position: u32) {}

    /// Open an editor on a new card that belongs at `position` in `deck`.
    fn new_card_at(&mut self, deck: Id, position: u32) {
        self.library_mut().selected_deck = Some(deck);
        self.library_mut().new_position = Some(position);
        self.new_card();
    }

    /// Move a card into a deck, keeping whatever schedule it has.
    fn move_card(&mut self, _card: Id, _deck: Id) {}

    /// Add a deck under the selected one, or at the top when none is selected.
    fn new_deck(&mut self) {}

    /// Rename a deck. A host with no store cannot, and says so by doing nothing.
    fn rename_deck(&mut self, _deck: Id, _name: &str) {}

    /// Set or clear a deck's template. A host with no store cannot.
    fn set_deck_template(&mut self, _deck: Id, _template: Option<&str>) {}

    /// Close the template field and send it. False when no deck's template was being edited.
    fn finish_template(&mut self) -> bool {
        match self.library_mut().take_template() {
            Some((deck, template)) => {
                self.set_deck_template(deck, template.as_deref());
                true
            }
            None => false,
        }
    }

    /// Clear a deck's template, sending nothing typed in the field.
    fn clear_deck_template(&mut self, deck: Id) {
        self.library_mut().close_template();
        self.set_deck_template(deck, None);
    }

    /// Open the template field on the selected deck.
    fn begin_template(&mut self) {
        if let Some(deck) = self.library().selected_deck {
            self.library_mut().start_template(deck);
        }
    }

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

    /// Order the list, and remember the order in the view being shown if there is one.
    ///
    /// The change chose this: a view remembers the order the user leaves it in, so selecting that
    /// view again does not undo a deliberate change. Only the order is written back — a search
    /// typed while a view is selected is a search, not an edit of the view.
    fn choose_sort(&mut self, sort: Sort) {
        self.library_mut().set_sort(sort);
        self.remember_order();
    }

    fn flip_dir(&mut self) {
        self.library_mut().flip_dir();
        self.remember_order();
    }

    /// Show only pinned cards, and remember that in the view being shown if there is one.
    fn toggle_pinned_only(&mut self) {
        self.library_mut().flip_pinned_only();
        self.remember_pinned();
    }

    fn remember_pinned(&mut self) {
        let Some(id) = self.library().selected_view else {
            return;
        };
        let Some(mut spec) = self
            .library()
            .snapshot()
            .views
            .iter()
            .find(|view| view.id == id)
            .map(|view| view.spec.clone())
        else {
            return;
        };
        spec.pinned_only = self.library().pinned_only;
        self.update_view(id, &spec);
    }

    fn remember_order(&mut self) {
        let Some(id) = self.library().selected_view else {
            return;
        };
        let Some(mut spec) = self
            .library()
            .snapshot()
            .views
            .iter()
            .find(|view| view.id == id)
            .map(|view| view.spec.clone())
        else {
            return;
        };
        spec.sort = self.library().sort;
        spec.dir = self.library().dir;
        self.update_view(id, &spec);
    }

    /// Close the view name field and save the current filter under it. False when no view was
    /// being named.
    fn finish_view_create(&mut self) -> bool {
        match self.library_mut().take_view_create() {
            Some(name) => {
                self.create_view(&name);
                true
            }
            None => false,
        }
    }

    /// Close the view name field and rename the view it was opened on.
    fn finish_view_rename(&mut self) -> bool {
        match self.library_mut().take_view_rename() {
            Some((id, name)) => {
                if name.trim().is_empty() {
                    // A view with no name could not be told from another, so an emptied field
                    // keeps the old name rather than sending a write the store would refuse.
                    return false;
                }
                self.rename_view(id, &name);
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

    /// Turn the card's own reverse setting on or off; `None` hands it back to its deck.
    fn set_card_reverse(&mut self, _card: Id, _reverse: Option<bool>) {}

    /// Turn a deck's reverse setting on or off; `None` hands it back to the deck above it.
    fn set_deck_reverse(&mut self, _deck: Id, _reverse: Option<bool>) {}

    /// Destroy everything in the trash. A host with no store leaves it alone.
    fn empty_trash(&mut self) {}

    /// Save the filter the list is showing under a name.
    fn create_view(&mut self, _name: &str) {}

    fn rename_view(&mut self, _id: Id, _name: &str) {}

    fn delete_view(&mut self, _id: Id) {}

    /// Whether a card is pinned.
    fn set_pinned(&mut self, _card: Id, _pinned: bool) {}

    /// Choose how a deck is read. The store answers by republishing, so a surface that has no
    /// store leaves it alone.
    fn set_view(&mut self, _deck: Id, _view: ViewMode) {}

    /// Replace a saved view's filter. A host with no store cannot.
    fn update_view(&mut self, _id: Id, _spec: &ViewSpec) {}

    /// Import the file the user chose. Answers the command sent, so a flow test can wait for
    /// its answer; a host with no store answers `None`.
    fn import_file(&mut self, _path: std::path::PathBuf) -> Option<u64> {
        None
    }

    /// Export the library, or its selected deck, to the file the user chose.
    fn export_file(&mut self, _path: std::path::PathBuf) -> Option<u64> {
        None
    }
}

impl Host for Library {
    fn library(&self) -> &Library {
        self
    }

    /// A fixture host has no store to ask, so it applies the choice to its own snapshot, which is
    /// what the store's republish would have done.
    fn set_view(&mut self, deck: Id, view: ViewMode) {
        Library::set_view(self, deck, view)
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
    /// How the list is ordered, which a shuffle replaces for as long as it is showing.
    pub sort: Sort,
    pub dir: SortDir,
    /// The seed of a shuffle, or `None` for the saved order. A shuffle is never written to a view.
    pub shuffle: Option<u64>,
    /// How many shuffles have been asked for, so pressing it again deals another hand.
    pub shuffles: u64,
    /// Whether only pinned cards are listed.
    pub pinned_only: bool,
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
    /// The deck whose template is being edited, if any.
    pub templating: Option<Id>,
    /// The template being typed, kept apart from the deck so a cancel costs nothing.
    pub template_buffer: TextEdit,
    /// The deck waiting for a destination, while the sidebar offers itself as one.
    pub moving: Option<Id>,
    /// The deck waiting for an answer to "archive this?", when it holds cards.
    pub confirming_archive: Option<Id>,
    /// The card waiting for a deck, while the sidebar offers itself as one.
    pub moving_card: Option<Id>,
    /// Which tile has its action menu open, if any.
    pub menu: Option<Id>,
    /// The width the grid was last laid out at, so the column count follows the window.
    pub grid_width: f32,
    /// The saved view the list is showing, if it is showing one.
    pub selected_view: Option<Id>,
    /// The deck scope in force: the tree's selection, or the one a view carries.
    pub view_scope: Option<Id>,
    /// The view whose name is being edited, if any.
    pub naming_view: Option<Id>,
    /// Whether a name is being typed for a view about to be saved.
    pub new_view: bool,
    /// The view name being typed, kept apart from the store so a cancel costs nothing.
    pub view_buffer: TextEdit,
    /// What the column view is looking for inside the card it reads.
    pub find: TextEdit,
    /// The line numbers that hold the find, in reading order.
    pub find_hits: Vec<usize>,
    /// The cards the notebook draws, in the deck's own order: `(position, id)`.
    pub notebook: Vec<Id>,
    /// Which hit is shown, counting from zero; only meaningful while `find_hits` is not empty.
    pub find_at: usize,
    /// The card whose whole text the preview draws, if one is open.
    pub peek: Option<Id>,
    /// Where a card opened from the notebook belongs in its deck, once it is created.
    pub new_position: Option<u32>,
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
            sort: Sort::default(),
            dir: Sort::default().natural(),
            shuffle: None,
            shuffles: 0,
            pinned_only: false,
            visible: Vec::new(),
            built_rows: AtomicUsize::new(0),
            confirming_empty: false,
            renaming: None,
            rename_buffer: TextEdit::default(),
            templating: None,
            template_buffer: TextEdit::default(),
            moving: None,
            confirming_archive: None,
            moving_card: None,
            menu: None,
            grid_width: 0.0,
            selected_view: None,
            view_scope: None,
            naming_view: None,
            new_view: false,
            view_buffer: TextEdit::default(),
            find: TextEdit::default(),
            find_hits: Vec::new(),
            notebook: Vec::new(),
            find_at: 0,
            peek: None,
            new_position: None,
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
    /// Read `deck` in `view` from now on. The whole library has no deck to ask, so it keeps the
    /// grid.
    pub fn set_view(&mut self, deck: Id, view: ViewMode) {
        let snapshot = Arc::make_mut(&mut self.snapshot);
        if let Some(found) = snapshot.decks.iter_mut().find(|each| each.id == deck) {
            found.view = view;
        }
        self.refresh();
    }

    /// Move the selection one card down the list it is showing, stopping at the last card.
    /// Select `card`, keeping the find describing the card that is now on screen.
    pub fn select_card(&mut self, card: Id) {
        self.selected_card = Some(card);
        self.find_in_current();
    }

    pub fn select_next(&mut self) {
        let Some(at) = self
            .visible
            .iter()
            .position(|card| Some(card.id) == self.selected_card)
        else {
            // Nothing is selected yet, so the first movement lands on the first card.
            if let Some(card) = self.visible.first().map(|card| card.id) {
                self.select_card(card);
            }
            return;
        };
        let next = (at + 1).min(self.visible.len().saturating_sub(1));
        if let Some(card) = self.visible.get(next).map(|card| card.id) {
            self.select_card(card);
        }
    }

    /// Move the selection one card up the list it is showing, stopping at the first card.
    pub fn select_previous(&mut self) {
        let Some(at) = self
            .visible
            .iter()
            .position(|card| Some(card.id) == self.selected_card)
        else {
            if let Some(card) = self.visible.first().map(|card| card.id) {
                self.select_card(card);
            }
            return;
        };
        let previous = at.saturating_sub(1);
        if let Some(card) = self.visible.get(previous).map(|card| card.id) {
            self.select_card(card);
        }
    }

    /// The key of the element that draws the selected card in the view in force, so the frame loop
    /// can bring it into view. The column view draws one card, which is always in view.
    pub fn selection_key(&self) -> Option<String> {
        let index = self
            .visible
            .iter()
            .position(|card| Some(card.id) == self.selected_card)?;
        match self.view_mode() {
            // The grid and the list are drawn only where they are scrolled to, so the row that
            // holds the card has to be built before the card itself can be brought into view.
            ViewMode::Grid => Some(format!("line-{}", index / self.grid_columns().max(1))),
            ViewMode::List => Some(format!("line-{index}")),
            ViewMode::Notebook => Some(format!("notebook-{}", self.visible[index].id)),
            ViewMode::Column => Some("column-card".to_string()),
        }
    }

    /// Show the whole card the selection is on, or put the preview away if it is already up.
    pub fn toggle_peek(&mut self, card: Id) {
        self.peek = match self.peek {
            Some(open) if open == card => None,
            _ => Some(card),
        };
    }

    /// The card the preview draws.
    pub fn peeked(&self) -> Option<&CardSummary> {
        let card = self.peek?;
        self.snapshot.cards.iter().find(|own| own.id == card)
    }

    /// Where a card added next to the selected one belongs: its deck, and the place in it. Only
    /// the notebook offers this, because only it shows the deck's own order.
    pub fn add_target(&self, below: bool) -> Option<(Id, u32)> {
        if self.view_mode() != ViewMode::Notebook {
            return None;
        }
        let card = self.selected_card?;
        // The place comes from the deck's own order, not from the list: the list may be filtered
        // or sorted, and the notebook reads the deck.
        let deck = self.snapshot.cards.iter().find(|own| own.id == card)?.deck;
        let order = self.deck_order(deck);
        let index = order.iter().position(|own| own.id == card)?;
        let place = if below { index + 1 } else { index };
        Some((deck, u32::try_from(place).ok()?))
    }

    /// The cards of one deck, in the order the notebook reads them.
    pub fn deck_order(&self, deck: Id) -> Vec<&CardSummary> {
        let mut cards: Vec<&CardSummary> = self
            .snapshot
            .cards
            .iter()
            .filter(|card| card.deck == deck)
            .collect();
        cards.sort_by_key(|card| (card.position, card.id));
        cards
    }

    /// Where the selected card would go if it moved one place: its deck, and the new place in it.
    /// `None` at either end of the deck, so the move keys do nothing there.
    pub fn move_target(&self, up: bool) -> Option<(Id, u32)> {
        let card = self.selected_card?;
        let deck = self.snapshot.cards.iter().find(|own| own.id == card)?.deck;
        let order = self.deck_order(deck);
        let index = order.iter().position(|own| own.id == card)?;
        let place = if up { index.checked_sub(1)? } else { index + 1 };
        if place >= order.len() {
            return None;
        }
        Some((card, u32::try_from(place).ok()?))
    }

    /// Reread the find against the card the column view shows. Called whenever the text or the
    /// card changes, so the count never describes a card that is no longer on screen.
    pub fn find_in_current(&mut self) {
        let Some(markdown) = self.column_card().map(|card| card.markdown.clone()) else {
            self.find_hits.clear();
            self.find_at = 0;
            return;
        };
        self.find_in_text(&markdown);
    }

    /// The line numbers of `card` that hold the find, folded so case and accents do not matter.
    pub fn find_in(&mut self, card: &CardSummary) {
        self.find_in_text(&card.markdown);
    }

    /// The same reading over a card's text, which is what the borrow allows when the card comes
    /// from this list.
    pub fn find_in_text(&mut self, markdown: &str) {
        let wanted = typr_core::store::folded(&self.find.text);
        self.find_hits = if wanted.is_empty() {
            Vec::new()
        } else {
            markdown
                .lines()
                .enumerate()
                .filter(|(_, line)| typr_core::store::folded(line).contains(&wanted))
                .map(|(index, _)| index)
                .collect()
        };
        self.find_at = 0;
    }

    /// The card the column view reads: the selected one when it is in the list, else the first.
    pub fn column_card(&self) -> Option<&CardSummary> {
        self.selected_card
            .and_then(|id| self.visible.iter().find(|card| card.id == id))
            .or_else(|| self.visible.first())
    }

    /// Step to the next find hit, wrapping, and say nothing when there is none.
    pub fn find_next(&mut self) {
        if !self.find_hits.is_empty() {
            self.find_at = (self.find_at + 1) % self.find_hits.len();
        }
    }

    /// Step to the previous find hit, wrapping.
    pub fn find_previous(&mut self) {
        if !self.find_hits.is_empty() {
            self.find_at = (self.find_at + self.find_hits.len() - 1) % self.find_hits.len();
        }
    }

    /// What the find bar says: how many hits there are and which one is shown, or that there is
    /// none.
    pub fn find_label(&self) -> String {
        if self.find.text.trim().is_empty() {
            return String::new();
        }
        match self.find_hits.len() {
            0 => "No match".to_string(),
            total => format!("{} of {total}", self.find_at.min(total - 1) + 1),
        }
    }

    /// The line the find is showing, so the reader can see what was found even where the card's
    /// own rendering puts it out of view.
    pub fn find_line(&self) -> Option<&str> {
        let card = self.column_card()?;
        let line = *self.find_hits.get(self.find_at)?;
        card.markdown.lines().nth(line)
    }

    /// How the selected deck is read. The whole library has no view of its own, so it reads as the
    /// grid, which is what every deck looked like before views existed.
    /// How many tile columns the grid is drawing at, from the width the scroll viewport last
    /// reported and the same fallback `grid_view` uses before the first layout.
    pub fn grid_columns(&self) -> usize {
        let width = if self.grid_width > 0.0 {
            self.grid_width
        } else {
            FALLBACK_GRID_WIDTH
        };
        columns_for(width)
    }

    pub fn view_mode(&self) -> ViewMode {
        self.selected_deck
            .and_then(|id| self.snapshot.decks.iter().find(|deck| deck.id == id))
            .map(|deck| deck.view)
            .unwrap_or_default()
    }

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
        self.find_hits.clear();
        self.find_at = 0;
        self.selected_deck = deck;
        self.view_scope = deck;
        // A view is a filter, and choosing a deck is choosing another one.
        self.selected_view = None;
        // A name field belongs to the deck it was opened on, so moving to another deck abandons
        // it: otherwise the only way out of the field is to rename the deck.
        if self.renaming != deck {
            self.renaming = None;
            self.rename_buffer = TextEdit::default();
        }
        self.refresh();
    }

    /// Drop everything a view was filtering by, so deleting one leaves the whole library.
    pub fn forget_filters(&mut self) {
        self.query = TextEdit::default();
        self.tag = None;
        self.mode = CardMode::Active;
        self.pinned_only = false;
        self.sort = Sort::default();
        self.dir = self.sort.natural();
        self.shuffle = None;
        self.selected_deck = None;
        self.view_scope = None;
        self.selected_view = None;
        self.refresh();
    }

    /// Open the name field on a deck, holding what the deck is called now.
    pub fn start_rename(&mut self, deck: Id) {
        if let Some(found) = self.snapshot.decks.iter().find(|own| own.id == deck) {
            let name = found.name.clone();
            self.rename_buffer.replace_text(name.clone());
            // Selected, so typing replaces the name instead of appending to it.
            self.rename_buffer.set_selection(0, name.len());
            self.renaming = Some(deck);
        }
    }

    /// The name being typed, for the input that shows it.
    pub fn rename_text(&self) -> &TextEdit {
        &self.rename_buffer
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

    /// Open the template field on a deck, filled with what that deck sets.
    pub fn start_template(&mut self, deck: Id) {
        if let Some(found) = self.snapshot.decks.iter().find(|own| own.id == deck) {
            self.template_buffer
                .replace_text(found.template.clone().unwrap_or_default());
            self.templating = Some(deck);
        }
    }

    /// The template being typed, for the input that shows it.
    pub fn template_text(&self) -> &TextEdit {
        &self.template_buffer
    }

    /// The template field, for the input that edits it.
    pub fn template_field(&mut self) -> &mut TextEdit {
        &mut self.template_buffer
    }

    /// Finish the edit, handing back the deck and what was typed: an emptied field is `None`, so a
    /// cleared template is a template cleared rather than a template of nothing.
    pub fn take_template(&mut self) -> Option<(Id, Option<String>)> {
        let deck = self.templating?;
        self.templating = None;
        let text = self.template_buffer.text.trim().to_string();
        Some((deck, (!text.is_empty()).then_some(text)))
    }

    /// Close the template field without sending anything.
    pub fn close_template(&mut self) {
        self.templating = None;
    }

    /// The views the user saved, by name.
    pub fn views(&self) -> &[SavedView] {
        &self.snapshot.views
    }

    /// Apply a saved view whole: its deck scope, search, tags, mode, pin filter and order.
    ///
    /// The deck tree's selection is cleared rather than set to the view's scope, because a view is
    /// a filter over the library and not a place in it; the scope still filters the list.
    pub fn select_view(&mut self, id: Id) {
        let Some(view) = self
            .snapshot
            .views
            .iter()
            .find(|view| view.id == id)
            .cloned()
        else {
            return;
        };
        self.selected_view = Some(id);
        self.selected_deck = None;
        self.view_scope = view.spec.deck;
        self.query.replace_text(view.spec.query.clone());
        self.tag = view.spec.tags.first().cloned();
        self.mode = view.spec.mode;
        self.pinned_only = view.spec.pinned_only;
        self.sort = view.spec.sort;
        self.dir = view.spec.dir;
        // A shuffle is a moment, not part of a view.
        self.shuffle = None;
        self.refresh();
    }

    /// Open the name field on a view, holding what it is called now.
    pub fn begin_view_rename(&mut self, id: Id) {
        if let Some(found) = self.snapshot.views.iter().find(|own| own.id == id) {
            let name = found.name.clone();
            self.view_buffer.replace_text(name.clone());
            self.view_buffer.set_selection(0, name.len());
            self.naming_view = Some(id);
            self.new_view = false;
        }
    }

    /// Open an empty name field for a view about to be saved from the current filter.
    pub fn begin_view_create(&mut self) {
        self.view_buffer.replace_text(String::new());
        self.new_view = true;
        self.naming_view = None;
    }

    /// The view name being typed, for the input that shows it.
    pub fn view_text(&self) -> &TextEdit {
        &self.view_buffer
    }

    /// The name field, for the input that edits it.
    pub fn view_field(&mut self) -> &mut TextEdit {
        &mut self.view_buffer
    }

    /// Finish a rename, handing back the view and what was typed. Taking it closes the field, so a
    /// rename the store refuses does not leave it open behind the failure.
    pub fn take_view_rename(&mut self) -> Option<(Id, String)> {
        let id = self.naming_view?;
        self.naming_view = None;
        Some((id, self.view_buffer.text.clone()))
    }

    /// Finish a save, handing back the name; an empty one cancels, because a view with no name
    /// could not be told from another.
    pub fn take_view_create(&mut self) -> Option<String> {
        if !self.new_view {
            return None;
        }
        self.new_view = false;
        let name = self.view_buffer.text.trim().to_string();
        (!name.is_empty()).then_some(name)
    }

    /// Close either name field without sending anything.
    pub fn close_view_name(&mut self) {
        self.new_view = false;
        self.naming_view = None;
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
        let mut cards = self.snapshot.cards_matching(&self.spec());
        if let Some(seed) = self.shuffle {
            cards = typr_core::store::shuffled(cards, seed);
        }
        self.visible = cards.into_iter().cloned().collect();
        // The notebook reads the deck's order, so it needs that order as cards move and leave.
        self.notebook = if self.view_mode() == ViewMode::Notebook {
            in_deck_order(self).iter().map(|card| card.id).collect()
        } else {
            Vec::new()
        };
    }

    /// Order the list.
    pub fn set_sort(&mut self, sort: Sort) {
        self.sort = sort;
        // A chosen order runs the way it reads; the arrow beside it is the way to run it backwards.
        self.dir = sort.natural();
        // A shuffle is a moment: choosing an order ends it.
        self.shuffle = None;
        self.refresh();
    }

    /// Run the order backwards.
    pub fn flip_dir(&mut self) {
        self.dir = self.dir.flipped();
        self.refresh();
    }

    /// Show only pinned cards, or stop doing so.
    pub fn flip_pinned_only(&mut self) {
        self.pinned_only = !self.pinned_only;
        self.refresh();
    }

    /// Deal the cards again, in an order no view keeps.
    pub fn shuffle_now(&mut self) {
        self.shuffles += 1;
        self.shuffle = Some(self.shuffles);
        self.refresh();
    }

    /// Whether the list is showing a shuffle rather than the view's order.
    pub fn shuffled(&self) -> bool {
        self.shuffle.is_some()
    }

    /// The order control's label: the current order, or the shuffle.
    pub fn sort_label(&self) -> &'static str {
        if self.shuffle.is_some() {
            "Shuffled"
        } else {
            self.sort.label()
        }
    }

    /// The filter the list is showing, built from what the controls and the selected view set.
    pub fn spec(&self) -> ViewSpec {
        ViewSpec {
            // The tree's selection when there is one, else the scope a saved view carries.
            deck: self.selected_deck.or(self.view_scope),
            query: self.query.text.clone(),
            tags: self.tag.iter().cloned().collect(),
            mode: self.mode,
            pinned_only: self.pinned_only,
            sort: self.sort,
            dir: self.dir,
        }
    }

    /// A snapshot with `count` cards in one deck, for scenes and for the virtualised-list check.
    /// Seed the sidebar with saved views, for scenes and tests.
    pub fn seed_views(&mut self, views: Vec<SavedView>) {
        let mut snapshot = (*self.snapshot).clone();
        snapshot.views = views;
        self.set_snapshot(std::sync::Arc::new(snapshot));
    }

    /// Mark a card as pinned in the seeded snapshot, for scenes and tests.
    pub fn pin(&mut self, card: Id) {
        let mut snapshot = (*self.snapshot).clone();
        if let Some(found) = snapshot.cards.iter_mut().find(|found| found.id == card) {
            found.pinned = true;
        }
        self.set_snapshot(std::sync::Arc::new(snapshot));
    }

    pub fn seeded(count: usize) -> Library {
        let deck = Deck {
            id: 1,
            parent: None,
            name: "Seeded".to_string(),
            archived: false,
            reverse: None,
            template: None,
            view: typr_core::library_view::ViewMode::Grid,
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
                position: id,
                kind: typr_core::store::CardKind::Text,
                image: None,
                caption: None,
                masks: Vec::new(),
                sides: 2,
                tags: Vec::new(),
                archived: false,
                reverse: None,
                trashed_at: None,
                updated_at: 0,
                created_at: 0,
                reviews: 0,
                pinned: false,
                preview: format!("Card {id}"),
                name: format!("Card {id}"),
                markdown: format!("Card {id}\n---\nCard {id}"),
                search_text: format!("card {id}"),
                prompts: u32::from(id % 3 != 0),
                unscheduled: u32::from(id % 3 == 1),
                due_at: (id % 3 == 2).then_some(-1),
                fields: Vec::new(),
                template: None,
                template_from: None,
                references: Vec::new(),
                backlinks: Vec::new(),
            })
            .collect();
        library.refresh();
        library
    }

    /// A library whose tiles differ: long first sides, tags, and cards in each schedule state, so a
    /// scene shows the grid the way real content lays it out.
    pub fn seeded_tiles() -> Library {
        let mut library = Library::seeded(9);
        {
            let cards = Arc::make_mut(&mut library.snapshot);
            let texts = [
                ("What is mitosis?", "Cell division."),
                (
                    "Name the four chambers of the heart, in the order blood reaches them",
                    "Right atrium, right ventricle, left atrium, left ventricle.",
                ),
                ("El subjuntivo de \"hablar\"", "hable."),
                (
                    "Explain osmosis in one sentence.",
                    "Water crossing a membrane.",
                ),
                (
                    "Which organelle makes ATP, and what does the inner membrane fold into?",
                    "The mitochondrion; cristae.",
                ),
                (
                    "¿Qué es la fotosíntesis?",
                    "Cómo las plantas hacen alimento.",
                ),
                (
                    "List the stages of mitosis.",
                    "Prophase, metaphase, anaphase, telophase.",
                ),
                ("What does a ribosome do?", "Builds proteins."),
                (
                    "Describe the difference between diffusion and active transport, with an example of each",
                    "Diffusion is passive; active transport spends ATP, as the sodium pump does.",
                ),
            ];
            for (card, (front, back)) in cards.cards.iter_mut().zip(texts) {
                card.markdown = format!("{front}\n---\n{back}");
                card.preview = front.to_string();
                card.search_text = format!("{front} {back}").to_lowercase();
            }
            cards.cards[0].tags = vec!["biology".to_string()];
            cards.cards[1].tags = vec!["biology".to_string(), "anatomy".to_string()];
            cards.cards[4].tags = vec!["cells".to_string()];
            cards.cards[6].tags = vec!["exam".to_string(), "biology".to_string()];
            // One card is asked from both ends, so a scene shows the marker it earns.
            cards.cards[2].reverse = Some(true);
        }
        // The list is a projection of the snapshot, so it has to be rebuilt from the fixture.
        library.refresh();
        library
    }

    /// A library whose decks are nested three deep, so a scene can show the indentation.
    pub fn seeded_nested() -> Library {
        let mut library = Library::seeded(2);
        {
            let snapshot = Arc::make_mut(&mut library.snapshot);
            snapshot.decks = vec![
                Deck {
                    id: 1,
                    parent: None,
                    name: "Biology".to_string(),
                    archived: false,
                    reverse: None,
                    template: None,
                    view: typr_core::library_view::ViewMode::Grid,
                },
                Deck {
                    id: 2,
                    parent: Some(1),
                    name: "Cell biology".to_string(),
                    archived: false,
                    reverse: None,
                    template: None,
                    view: typr_core::library_view::ViewMode::Grid,
                },
                Deck {
                    id: 3,
                    parent: Some(2),
                    name: "Mitosis".to_string(),
                    archived: false,
                    reverse: None,
                    template: None,
                    view: typr_core::library_view::ViewMode::Grid,
                },
            ];
        }
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

/// Decks in the order the sidebar draws them: every deck owns the rows below it, and each
/// level is indented. A deck whose parent is missing counts as a root, and a cycle has no root
/// at all, so the leftovers are appended rather than hidden.
pub(crate) fn deck_tree(decks: &[Deck]) -> Vec<(&Deck, usize)> {
    let known: HashSet<Id> = decks.iter().map(|deck| deck.id).collect();
    let mut ordered = Vec::with_capacity(decks.len());
    let mut drawn = HashSet::with_capacity(decks.len());
    for deck in decks {
        if deck.parent.is_none_or(|parent| !known.contains(&parent)) {
            push_branch(deck, 0, decks, &mut drawn, &mut ordered);
        }
    }
    for deck in decks {
        if !drawn.contains(&deck.id) {
            push_branch(deck, 0, decks, &mut drawn, &mut ordered);
        }
    }
    ordered
}

/// Appends one deck and then its children, depth first.
fn push_branch<'a>(
    deck: &'a Deck,
    depth: usize,
    decks: &'a [Deck],
    drawn: &mut HashSet<Id>,
    ordered: &mut Vec<(&'a Deck, usize)>,
) {
    if !drawn.insert(deck.id) {
        return;
    }
    ordered.push((deck, depth));
    for child in decks.iter().filter(|child| child.parent == Some(deck.id)) {
        push_branch(child, depth + 1, decks, drawn, ordered);
    }
}

fn deck_row<S: Host>(
    deck: &Deck,
    depth: usize,
    name: &TextEdit,
    selected: bool,
    renaming: bool,
    moving: bool,
) -> El {
    let t = tok::theme(true);
    let id = deck.id;
    let inset = 10.0 + depth as f32 * 12.0;
    if renaming {
        return row()
            .key(format!("deck-{}", deck.id))
            .h(30.0)
            .px(inset)
            .center_y()
            .fill(t.control_off)
            .child(
                text_input(name, type_::LABEL)
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
        .px(inset)
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
        .w(crate::sidebar::WIDTH)
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
    for (deck, depth) in deck_tree(&library.snapshot.decks) {
        sidebar = sidebar.child(deck_row::<S>(
            deck,
            depth,
            library.rename_text(),
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
    sidebar = sidebar.child(views_section::<S>(s));
    if library.selected_deck.is_some() {
        sidebar = sidebar
            .child(
                Button::new("Rename")
                    .variant(ButtonVariant::Ghost)
                    // The field only exists once `renaming` is set, and `Tx::focus` resolves the
                    // key once the change that set it is on screen, so the rename can be typed
                    // straight after the press without reaching for the mouse.
                    .on_press_tx(|s: &mut S, tx: &mut rune::Tx| {
                        s.begin_rename();
                        tx.focus("deck-name", true);
                    })
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
            )
            .child(deck_reverse::<S>(s))
            .child(
                Button::new("Template")
                    .variant(ButtonVariant::Ghost)
                    .on_press_tx(|s: &mut S, tx: &mut rune::Tx| {
                        s.begin_template();
                        tx.focus("template-field", true);
                    })
                    .build(&KIT)
                    .key("edit-template"),
            );
        if let Some(deck) = library.templating {
            sidebar = sidebar.child(template_panel::<S>(s, deck));
        }
    }
    sidebar
}

/// The saved views, under the decks: one row each, and the field that names one.
fn views_section<S: Host>(s: &S) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let mut section = col().key("views").w_full().gap(2.0);
    section = section.child(
        text("Views")
            .key("views-title")
            .font(type_::LABEL)
            .color(t.text_secondary),
    );
    for view in library.views() {
        section = section.child(view_row::<S>(s, view));
    }
    section = section.child(
        text(if library.pinned_only {
            "★ Pinned only"
        } else {
            "☆ Pinned only"
        })
        .key("pinned-only")
        .font(type_::MONO_CAPTION)
        .color(if library.pinned_only {
            tok::accent(0, true)
        } else {
            t.text_secondary
        })
        .on_press(|s: &mut S| s.toggle_pinned_only()),
    );
    if library.new_view || library.naming_view.is_some() {
        let saving = library.new_view;
        section = section.child(
            row()
                .key("view-name-row")
                .w_full()
                .h(30.0)
                .px(10.0)
                .center_y()
                .fill(t.control_off)
                .child(
                    text_input(library.view_text(), type_::LABEL)
                        .key("view-name")
                        .single_line()
                        .grow(1.0)
                        .placeholder("Name this view", t.text_secondary)
                        .on_edit(|s: &mut S| s.library_mut().view_field())
                        .on_submit(move |s: &mut S, _| {
                            if saving {
                                s.finish_view_create();
                            } else {
                                s.finish_view_rename();
                            }
                        }),
                ),
        );
    }
    section = section.child(
        Button::new("New view")
            .variant(ButtonVariant::Ghost)
            .on_press_tx(|s: &mut S, tx: &mut rune::Tx| {
                s.library_mut().begin_view_create();
                tx.focus("view-name", true);
            })
            .build(&KIT)
            .key("new-view"),
    );
    if library.selected_view.is_some() {
        section = section
            .child(
                Button::new("Rename view")
                    .variant(ButtonVariant::Ghost)
                    .on_press_tx(|s: &mut S, tx: &mut rune::Tx| {
                        if let Some(id) = s.library().selected_view {
                            s.library_mut().begin_view_rename(id);
                            tx.focus("view-name", true);
                        }
                    })
                    .build(&KIT)
                    .key("rename-view"),
            )
            .child(
                Button::new("Delete view")
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut S| {
                        if let Some(id) = s.library().selected_view {
                            s.delete_view(id);
                        }
                    })
                    .build(&KIT)
                    .key("delete-view"),
            );
    }
    section
}

/// One saved view's row, and a note when the deck it was saved on is gone.
fn view_row<S: Host>(s: &S, view: &SavedView) -> El {
    let t = tok::theme(true);
    let library = s.library();
    let id = view.id;
    let selected = library.selected_view == Some(id);
    let mut label = row().w_full().gap(4.0).child(
        text(view.name.clone())
            .font(type_::BODY)
            .color(if selected { rune::Color::WHITE } else { t.text }),
    );
    if view.scope_missing(library.snapshot()) {
        label = label.child(
            text("its deck is gone")
                .key("view-scope-missing")
                .font(type_::MONO_CAPTION)
                .color(if selected {
                    rune::Color::WHITE
                } else {
                    t.text_secondary
                }),
        );
    }
    row()
        .key(format!("view-{}", view.id))
        .h(30.0)
        .px(10.0)
        .radius(6.0)
        .center_y()
        .fill(if selected {
            tok::accent(0, true).alpha(0.85)
        } else {
            t.control_off.alpha(0.0)
        })
        .child(label)
        .on_press(move |s: &mut S| s.library_mut().select_view(id))
}

/// A deck's template, with a preview of one of the cards it would render.
fn template_panel<S: Host>(s: &S, deck: Id) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let Some(found) = library.snapshot().decks.iter().find(|own| own.id == deck) else {
        return col().key("template-panel");
    };
    // One card of the deck, rendered as the template in the field would render it, so the author
    // sees the effect before saving.
    let preview = library
        .snapshot()
        .cards
        .iter()
        .find(|card| card.deck == deck)
        .map(|card| {
            typr_core::template::render_card(
                &library.template_text().text,
                &card.fields,
                Some(&found.name),
            )
        });
    col()
        .key("template-panel")
        .w_full()
        .gap(6.0)
        .padding(6.0)
        .radius(6.0)
        .fill(t.control_off)
        .child(
            text("Deck template")
                .key("template-title")
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(
            text_input(library.template_text(), type_::MONO_CAPTION.lh(18.0))
                .key("template-field")
                .w_full()
                .h(96.0)
                .placeholder(
                    "<<Front>>\n---\n<<Back>>, one separator for the deck",
                    t.text_secondary,
                )
                .on_edit(|s: &mut S| s.library_mut().template_field())
                .on_submit(|s: &mut S, _| {
                    s.finish_template();
                }),
        )
        .child(
            row()
                .w_full()
                .gap(6.0)
                .child(
                    Button::new("Save")
                        .variant(ButtonVariant::Submit)
                        .on_press(|s: &mut S| {
                            s.finish_template();
                        })
                        .build(&KIT)
                        .key("template-save"),
                )
                .child(
                    Button::new("Clear")
                        .variant(ButtonVariant::Ghost)
                        .on_press(move |s: &mut S| s.clear_deck_template(deck))
                        .build(&KIT)
                        .key("template-clear"),
                )
                .child(
                    Button::new("Close")
                        .variant(ButtonVariant::Ghost)
                        .on_press(|s: &mut S| s.library_mut().close_template())
                        .build(&KIT)
                        .key("template-close"),
                ),
        )
        .child(match preview {
            Some(shown) => col()
                .key("template-preview")
                .w_full()
                .padding(6.0)
                .radius(4.0)
                .fill(t.card)
                .child(
                    Markdown::new(crate::editor::preview_body(&shown))
                        .render(&MarkdownStyle::neutral(true)),
                ),
            None => text("No cards in this deck yet.")
                .key("template-preview-empty")
                .font(type_::LABEL)
                .color(t.text_secondary),
        })
}

/// The deck's three-state reverse setting: on, off, or whatever its parent says.
fn deck_reverse<S: Host>(s: &S) -> El {
    let library = s.library();
    let deck = library
        .selected_deck
        .and_then(|id| library.snapshot().decks.iter().find(|deck| deck.id == id));
    let Some(deck) = deck else {
        return row().key("deck-reverse");
    };
    let id = deck.id;
    let state = match deck.reverse {
        Some(true) => "Both ways",
        Some(false) => "One way",
        None => "Inherit",
    };
    row()
        .key("deck-reverse")
        .w_full()
        .gap(4.0)
        .child(
            text(format!("Reverse: {state}"))
                .font(type_::LABEL)
                .color(tok::theme(true).text_secondary),
        )
        .child(deck_reverse_button::<S>("On", "reverse-on", id, Some(true)))
        .child(deck_reverse_button::<S>(
            "Off",
            "reverse-off",
            id,
            Some(false),
        ))
        .child(deck_reverse_button::<S>(
            "Inherit",
            "reverse-inherit",
            id,
            None,
        ))
}

fn deck_reverse_button<S: Host>(label: &str, key: &str, deck: Id, reverse: Option<bool>) -> El {
    Button::new(label)
        .variant(ButtonVariant::Ghost)
        .on_press(move |s: &mut S| s.set_deck_reverse(deck, reverse))
        .build(&KIT)
        .key(key)
}

/// The order control: one button per order, the one in force marked, and a shuffle beside them.
///
/// The shuffle is a separate button because it is not an order a view can keep: pressing an order
/// ends the shuffle, and pressing the shuffle never writes anything.
/// The order controls, or — for the views that read the deck's own order — a word saying so, so
/// the bar never offers an order it does not apply.
fn order_controls<S: Host>(s: &S) -> El {
    match s.library().view_mode() {
        ViewMode::Grid | ViewMode::List => sort_controls::<S>(s),
        ViewMode::Notebook | ViewMode::Column => row().key("sorting").h(28.0).gap(8.0).child(
            text("Deck order")
                .font(type_::MONO_CAPTION)
                .color(tok::theme(true).text_secondary),
        ),
    }
}

fn sort_controls<S: Host>(s: &S) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let mut row = row().key("sorting").gap(2.0).center_y();
    for sort in Sort::ALL {
        let chosen = !library.shuffled() && library.sort == sort;
        row = row.child(
            Button::new(sort.label())
                .variant(if chosen {
                    ButtonVariant::Submit
                } else {
                    ButtonVariant::Ghost
                })
                .on_press(move |s: &mut S| s.choose_sort(sort))
                .build(&KIT)
                .key(format!("sort-{}", sort.as_str())),
        );
    }
    row.child(
        Button::new(if library.shuffled() {
            "Again"
        } else {
            "Shuffle"
        })
        .variant(if library.shuffled() {
            ButtonVariant::Submit
        } else {
            ButtonVariant::Ghost
        })
        .on_press(|s: &mut S| s.library_mut().shuffle_now())
        .build(&KIT)
        .key("sort-shuffle"),
    )
    .child(
        text(match library.dir {
            SortDir::Ascending => "Ascending",
            SortDir::Descending => "Descending",
        })
        .key("sort-dir")
        .font(type_::MONO_CAPTION)
        .color(t.text_secondary)
        .on_press(|s: &mut S| s.flip_dir()),
    )
}

/// The four ways a deck can be read, with the one in force marked. The whole library has no deck
/// to choose for, so it draws nothing.
fn view_switcher<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let library = s.library();
    let Some(deck) = library.selected_deck else {
        return row().key("view-switcher").h(ROW_HEIGHT);
    };
    let current = library.view_mode();
    let mut row_ = row()
        .key("view-switcher")
        .h(ROW_HEIGHT)
        .px(12.0)
        .center_y()
        .gap(4.0)
        .child(
            text(format!("View: {}", current.label()))
                .key("view-name")
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        );
    for view in ViewMode::ALL {
        let mut button = Button::new(view.label()).variant(ButtonVariant::Ghost);
        if view == current {
            button = button.variant(ButtonVariant::Submit);
        }
        row_ = row_.child(
            button
                .on_press(move |s: &mut S| s.set_view(deck, view))
                .build(&KIT)
                .key(format!("view-{}", view.as_str())),
        );
    }
    row_
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
        .child(order_controls::<S>(s))
        .child(
            text(
                match (library.visible.len(), library.snapshot.cards.len()) {
                    (shown, total) if shown == total => format!("{total} {}", plural(total)),
                    (shown, total) => format!("{shown} of {total}"),
                },
            )
            .font(type_::LABEL)
            .color(t.text_secondary),
        );
    if let Some(deck) = library.selected_deck
        && library
            .snapshot
            .decks
            .iter()
            .any(|known| known.id == deck && known.archived)
    {
        bar = bar.child(
            Button::new("Unarchive deck")
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| s.unarchive_deck(deck))
                .build(&KIT)
                .key("unarchive-deck"),
        );
    }
    if library.mode == CardMode::Trash {
        bar = bar.child(empty_trash_button::<S>());
    }
    let mut new_card = Button::new("New card").variant(ButtonVariant::Submit);
    if let Some(chord) = crate::keymap::chord(Surface::Library, Command::NewCard)
        && s.keycaps()
    {
        new_card = new_card.shortcut(chord.to_string());
    }
    bar = bar.child(
        new_card
            .on_press(|s: &mut S| s.new_card())
            .build(&KIT)
            // The empty deck's tile owns `new-card`; this bar button is the other way in.
            .key("new-card-button"),
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
    bar = bar.child(
        Button::new("Settings")
            .variant(ButtonVariant::Ghost)
            .on_press(|s: &mut S| s.open_settings())
            .build(&KIT)
            .key("settings"),
    );
    bar = bar.child(
        Button::new("Import")
            .variant(ButtonVariant::Ghost)
            .on_press_tx(|_: &mut S, tx: &mut Tx| {
                tx.open_file(import_dialog(), |s: &mut S, result, _| {
                    if let Ok(Some(path)) = result {
                        let _ = s.import_file(path);
                    }
                });
            })
            .build(&KIT)
            .key("import"),
    );
    bar = bar.child(
        Button::new("Export")
            .variant(ButtonVariant::Ghost)
            .on_press_tx(|_: &mut S, tx: &mut Tx| {
                tx.save_file(export_dialog(), |s: &mut S, result, _| {
                    if let Ok(Some(path)) = result {
                        let _ = s.export_file(path);
                    }
                });
            })
            .build(&KIT)
            .key("export"),
    );
    bar
}

/// One row of the card list.
/// The formats an import can read, offered by the library bar and the settings screen alike.
pub fn import_dialog() -> FileDialogOptions {
    FileDialogOptions {
        title: "Import cards".to_string(),
        ..FileDialogOptions::default()
    }
    .filter("Mochi export", &["mochi"])
    .filter("Spreadsheet", &["csv"])
    .filter("Markdown", &["md", "markdown"])
}

/// The formats an export can write.
pub fn export_dialog() -> FileDialogOptions {
    FileDialogOptions {
        title: "Export cards".to_string(),
        ..FileDialogOptions::default()
    }
    .filter("typr library", &["typr"])
    .filter("Spreadsheet", &["csv"])
    .filter("Markdown", &["md"])
}

/// "card" for one, "cards" for everything else, so the count never reads "1 cards".
fn plural(count: usize) -> &'static str {
    if count == 1 { "card" } else { "cards" }
}

/// The library's own movement keys: move the selection and ask the runtime to bring the card it
/// landed on into view. Answers whether it took the key; every other library key is the app's.
pub fn key<S: Host>(host: &mut S, event: &rune::KeyEvent, tx: &mut rune::Tx) -> bool {
    let Some(command) = crate::keymap::command_for(Surface::Library, event) else {
        return false;
    };
    match command {
        Command::SelectionNext | Command::SelectionPrevious => {
            match command {
                Command::SelectionNext => host.library_mut().select_next(),
                _ => host.library_mut().select_previous(),
            }
            if let Some(key) = host.library().selection_key() {
                tx.scroll_into_view(key);
            }
            true
        }
        Command::EditCard => match host.library().selected_card {
            Some(card) => {
                host.open_card(card);
                true
            }
            None => false,
        },
        Command::PreviewCard => match host.library().selected_card {
            Some(card) => {
                host.library_mut().toggle_peek(card);
                true
            }
            None => false,
        },
        Command::DeleteCard => match host.library().selected_card {
            Some(card) => {
                host.trash_card(card);
                true
            }
            None => false,
        },
        Command::MoveCardUp | Command::MoveCardDown => {
            let up = command == Command::MoveCardUp;
            match host.library().move_target(up) {
                Some((card, position)) => {
                    host.reorder_card(card, position);
                    true
                }
                None => false,
            }
        }
        Command::AddCardBelow | Command::AddCardAbove => {
            let below = command == Command::AddCardBelow;
            match host.library().add_target(below) {
                Some((deck, position)) => {
                    host.new_card_at(deck, position);
                    true
                }
                None => false,
            }
        }
        _ => false,
    }
}

/// The library screen.
pub fn view<S: Host>(s: &S) -> El {
    let list = view_body::<S>(s);
    let body = row()
        .w_full()
        .h_full()
        .fill(tok::theme(true).bg)
        .on_layout(|s: &mut S, rect: [f32; 4]| {
            // The window, less the sidebar: measuring the grid's own parent would make the tiles
            // decide the width they are laid out in.
            let width = (rect[2] - crate::sidebar::WIDTH).max(0.0);
            if (s.library().grid_width - width).abs() > 0.5 {
                s.library_mut().grid_width = width;
            }
        })
        .child(deck_sidebar::<S>(s))
        .child(
            col()
                .grow(1.0)
                .h_full()
                .child(filter_bar::<S>(s))
                .child(view_switcher::<S>(s))
                .child(list),
        );
    let mut layer = Vec::new();
    confirmation::<S>(s, &mut layer);
    deck_confirmation::<S>(s, &mut layer);
    peek_panel::<S>(s, &mut layer);
    root(body, layer)
}

/// The list area, in whichever view the selected deck is read.
fn view_body<S: Host>(s: &S) -> El {
    match s.library().view_mode() {
        ViewMode::Grid => grid_view::<S>(s),
        ViewMode::List => list_view::<S>(s),
        ViewMode::Notebook => notebook_view::<S>(s),
        ViewMode::Column => column_view::<S>(s),
    }
}

/// The deck's cards as tiles, as many across as the window holds.
fn grid_view<S: Host>(s: &S) -> El {
    let library = s.library();
    let now = typr_core::clock::Clock::now_ms(&typr_core::clock::SystemClock);
    // The grid is laid out from the width the scroll viewport actually got, so the column count
    // follows the window rather than a constant.
    let width = if library.grid_width > 0.0 {
        library.grid_width
    } else {
        FALLBACK_GRID_WIDTH
    };
    let columns = library.grid_columns();
    let tile_width = tile_width(width, columns);
    // An empty deck still draws one line: that line holds the tile that starts the first card.
    let lines = if library.visible.is_empty() {
        1
    } else {
        tile_rows(library.visible.len(), columns)
    };
    col()
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
                    lines,
                    TILE_HEIGHT + GAP,
                    |_s: &S, index| -> Cow<'_, str> { format!("line-{index}").into() },
                    move |s: &S, index| {
                        s.library().built_rows.fetch_add(1, Ordering::Relaxed);
                        let library = s.library();
                        let mut line = row().key("line").w_full().gap(GAP).items_start();
                        if library.visible.is_empty() {
                            return line.child(self::tile::placeholder::<S>(tile_width));
                        }
                        let first = index * columns;
                        let last = (first + columns).min(library.visible.len());
                        for card in &library.visible[first..last] {
                            line = line.child(self::tile::tile::<S>(s, card, tile_width, now));
                        }
                        line
                    },
                ),
        )
}

/// One line per card: the anatomy a version-11 library had, kept as its own view.
fn list_view<S: Host>(s: &S) -> El {
    let library = s.library();
    let now = typr_core::clock::Clock::now_ms(&typr_core::clock::SystemClock);
    let lines = library.visible.len();
    col()
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
                    lines,
                    ROW_HEIGHT,
                    |_s: &S, index| -> Cow<'_, str> { format!("line-{index}").into() },
                    move |s: &S, index| {
                        s.library().built_rows.fetch_add(1, Ordering::Relaxed);
                        list_row::<S>(s, index, now)
                    },
                ),
        )
}

/// One card as one line: what it is called, what it opens with, and where its schedule stands.
fn list_row<S: Host>(s: &S, index: usize, now: i64) -> El {
    let t = tok::theme(true);
    let library = s.library();
    let Some(card) = library.visible.get(index) else {
        return row().key("line").h(ROW_HEIGHT).w_full();
    };
    let id = card.id;
    let (word, kicker) = self::tile::words(self::tile::state(card, now));
    let mut line = row()
        .w_full()
        .h(ROW_HEIGHT)
        .px(12.0)
        .center_y()
        .gap(8.0)
        .fill(if library.selected_card == Some(id) {
            t.control_off
        } else {
            t.bg
        })
        .child(
            text(card.name.clone())
                .font(type_::LABEL)
                .shrink()
                .color(t.text),
        )
        .child(
            text(card.preview.clone())
                .font(type_::BODY)
                .grow(1.0)
                .shrink()
                .color(t.text_secondary),
        )
        .child(text(word).font(type_::MONO_CAPTION).color(t.text_secondary));
    if card.pinned {
        line = line.child(text("\u{2605}").color(t.text));
    }
    let _ = kicker;
    // The virtual row keys the element the builder returns, so the line that carries the card's
    // key is one level in: a key on the returned element would be replaced by `line-{index}`.
    col().w_full().h(ROW_HEIGHT).child(
        line.on_press(move |s: &mut S| {
            s.library_mut().select_card(id);
            s.open_card(id);
        })
        // The key is set last because a press target wraps its element, and the wrapper is what the
        // elements list shows.
        .key(format!("list-row-{id}")),
    )
}

/// Every card's contents stacked, which is the view a deck of long cards is read in.
fn notebook_view<S: Host>(s: &S) -> El {
    // Only the cards in the window are built, so a five-thousand-card deck costs what the screen
    // holds: a row's measured height is kept by key, and the estimate only places rows nobody has
    // reached yet.
    let count = s.library().notebook.len();
    col()
        .key("cards")
        .grow(1.0)
        .h_full()
        .gap(GAP)
        .padding(16.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("notebook-rows")
                .growth(GrowthSpec::default())
                .virtual_rows(
                    count,
                    NOTEBOOK_ESTIMATE,
                    |s: &S, index: usize| match s.library().notebook.get(index) {
                        Some(id) => Cow::Owned(format!("notebook-{id}")),
                        None => Cow::Borrowed("notebook-gone"),
                    },
                    |s: &S, index: usize| match s.library().notebook_card_at(index) {
                        Some(card) => notebook_card::<S>(s, card),
                        None => col().key("notebook-gone"),
                    },
                ),
        )
}

/// The cards of the shown deck in the order the deck holds them.
fn in_deck_order(library: &Library) -> Vec<&CardSummary> {
    let mut cards: Vec<&CardSummary> = library.visible.iter().collect();
    cards.sort_by_key(|card| (card.position, card.id));
    cards
}

impl Library {
    /// The notebook's card at `index` in the order the deck holds it.
    pub fn notebook_card_at(&self, index: usize) -> Option<&CardSummary> {
        let id = *self.notebook.get(index)?;
        self.visible.iter().find(|card| card.id == id)
    }
}

/// What a card reads as in the notebook and the column: its text with every answer masked, or —
/// for a diagram, whose text is empty — its caption.
fn body_text(card: &CardSummary) -> El {
    match card.kind {
        typr_core::store::CardKind::Diagram => text(card.caption.clone().unwrap_or_default())
            .key(format!("notebook-caption-{}", card.id))
            .font(type_::BODY)
            .color(tok::theme(true).text),
        typr_core::store::CardKind::Text => {
            Markdown::new(crate::editor::preview_body(&card.markdown))
                .render(&MarkdownStyle::neutral(true))
        }
    }
}

/// How tall a notebook card is guessed to be before it has been laid out once.
const NOTEBOOK_ESTIMATE: f32 = 220.0;

/// One card in the notebook: its name, every side it holds, and the way into its editor.
fn notebook_card<S: Host>(s: &S, card: &CardSummary) -> El {
    let t = tok::theme(true);
    let id = card.id;
    let now = typr_core::clock::Clock::now_ms(&typr_core::clock::SystemClock);
    let (word, _) = self::tile::words(self::tile::state(card, now));
    let mut body = col()
        .key(format!("notebook-{id}"))
        .w_full()
        .max_w(NOTEBOOK_WIDTH)
        .gap(8.0)
        .padding(14.0)
        .radius(6.0)
        .fill(if s.library().selected_card == Some(id) {
            t.control_off
        } else {
            t.card
        })
        .child(
            row()
                .key(format!("notebook-head-{id}"))
                .w_full()
                .center_y()
                .gap(8.0)
                .child(
                    text(card.name.clone())
                        .key(format!("notebook-name-{id}"))
                        .font(type_::LABEL)
                        .grow(1.0)
                        .color(t.text),
                )
                .child(text(word).font(type_::MONO_CAPTION).color(t.text_secondary))
                .child(
                    Button::new("Edit")
                        .variant(ButtonVariant::Ghost)
                        .on_press(move |s: &mut S| {
                            s.library_mut().select_card(id);
                            s.open_card(id);
                        })
                        .build(&KIT)
                        .key(format!("notebook-edit-{id}")),
                ),
        )
        // A card with no separator holds one side, and the notebook shows what is there rather
        // than inventing a back.
        .child(body_text(card));
    if !card.tags.is_empty() {
        body = body.child(
            text(card.tags.join(" \u{b7} "))
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        );
    }
    body.on_press(move |s: &mut S| s.library_mut().select_card(id))
}

/// One card's contents at reading width, with a find over it: the layout for working through a
/// long card.
fn column_view<S: Host>(s: &S) -> El {
    let library = s.library();
    let t = tok::theme(true);
    let Some(card) = library.column_card() else {
        return col().key("cards").grow(1.0).h_full().child(
            text("No card to read")
                .color(t.text_secondary)
                .key("column-empty"),
        );
    };
    // The find bar sits outside the scroll view: a scrolled child is laid out unbounded across, so
    // its `w_full` would be its own content and the buttons would land off the window.
    let mut body = col()
        .key("cards")
        .grow(1.0)
        .h_full()
        .child(column_find::<S>(s))
        .child(
            col()
                .grow(1.0)
                .h_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    col()
                        .key("column-card")
                        .w_full()
                        .max_w(COLUMN_WIDTH)
                        .gap(10.0)
                        .padding(24.0)
                        .child(
                            text(card.name.clone())
                                .key("column-name")
                                .font(type_::TITLE)
                                .color(t.text),
                        )
                        .child(body_text(card)),
                ),
        );
    if let Some(line) = library.find_line() {
        body = body.child(
            text(line.to_string())
                .key("column-hit")
                .font(type_::BODY)
                .color(t.text_secondary)
                .w_full()
                .max_w(COLUMN_WIDTH),
        );
    }
    body
}

/// The find bar the column view reads with: a field, the count, and the two ways through the hits.
fn column_find<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let library = s.library();
    let mut bar = row()
        .key("column-find")
        .w_full()
        .h(44.0)
        .px(24.0)
        .center_y()
        .gap(8.0)
        .child(
            // A fixed width, not `grow`: the bar is measured by its content, and a growing field
            // in a row with no bound pushed the two buttons past the window's edge.
            text_input(&library.find, type_::BODY.lh(20.0))
                .key("find")
                .single_line()
                .w(240.0)
                .placeholder("Find in this card", t.text_secondary)
                .on_edit(|s: &mut S| &mut s.library_mut().find)
                .on_input_change(|s: &mut S, _| s.library_mut().find_in_current()),
        )
        .child(
            text(library.find_label())
                .key("find-count")
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        )
        .child(
            Button::new("Previous")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.library_mut().find_previous())
                .build(&KIT)
                .key("find-previous"),
        )
        .child(
            Button::new("Next")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut S| s.library_mut().find_next())
                .build(&KIT)
                .key("find-next"),
        );
    if library.find_hits.is_empty() && !library.find.text.trim().is_empty() {
        bar = bar.child(text("No match").key("find-none").color(t.text_secondary));
    }
    bar
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
/// The whole text of the selected card, so a preview costs no trip to the editor.
fn peek_panel<S: Host>(s: &S, layer: &mut Vec<El>) {
    let Some(card) = s.library().peeked() else {
        return;
    };
    let body = col()
        .key("peek-body")
        .w_full()
        .gap(12.0)
        .child(
            text(card.name.clone())
                .key("peek-name")
                .font(type_::TITLE)
                .max_w(560.0),
        )
        .child(
            Markdown::new(crate::editor::preview_body(&card.markdown))
                .render(&MarkdownStyle::neutral(true)),
        );
    Dialog::new("peek", "Preview")
        .description("P puts this away.")
        .fit()
        .open(true)
        .on_close(|s: &mut S| s.library_mut().peek = None)
        .child(body)
        .build(&KIT, layer);
}

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

pub mod tile;

#[cfg(test)]
mod tests;

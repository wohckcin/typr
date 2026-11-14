//! Saved filters and the orders a list can be shown in.
//!
//! One type, [`ViewSpec`], is what the sidebar, a saved view and a test all build: a deck scope, a
//! search text, a set of tags, a mode, a pinned-only flag and an order. The list filters and sorts a
//! [`Snapshot`](super::Snapshot) through it, so adding a filter means adding a field here rather
//! than a parameter to every caller.

use super::{CardMode, CardSummary, Id, Snapshot, folded};

/// How a list is ordered. Every order but the shuffle is a property of a saved view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// The last one touched first, which is what a library shows before anything is chosen.
    #[default]
    LastEdited,
    Created,
    /// By the text of the first side, folded so case and accents do not decide the order.
    FirstSide,
    /// The earliest due prompt first; a card with nothing scheduled comes last either way.
    DueDate,
    Reviews,
}

impl Sort {
    /// Every order the control offers, in the order it offers them.
    pub const ALL: [Sort; 5] = [
        Sort::LastEdited,
        Sort::Created,
        Sort::FirstSide,
        Sort::DueDate,
        Sort::Reviews,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Sort::LastEdited => "edited",
            Sort::Created => "created",
            Sort::FirstSide => "first-side",
            Sort::DueDate => "due",
            Sort::Reviews => "reviews",
        }
    }

    /// The label the sort control shows.
    pub fn label(self) -> &'static str {
        match self {
            Sort::LastEdited => "Last edited",
            Sort::Created => "Created",
            Sort::FirstSide => "First side",
            Sort::DueDate => "Due date",
            Sort::Reviews => "Reviews",
        }
    }

    /// `None` for a key no build of typr writes, which reads as the default rather than an error:
    /// an order is a preference, not library data worth refusing to open for.
    pub fn parse(text: &str) -> Option<Sort> {
        Sort::ALL.into_iter().find(|sort| sort.as_str() == text)
    }

    /// The cards in this order. A stable sort, so cards that compare equal keep the order the
    /// snapshot gave them, and the result is a new vector rather than a mutation of the snapshot.
    pub fn order<'a>(&self, cards: Vec<&'a CardSummary>, dir: SortDir) -> Vec<&'a CardSummary> {
        let mut cards = cards;
        // A card with nothing scheduled is not early or late: it comes last in both directions, so
        // the sort is not of the whole list but of its dated part.
        if *self == Sort::DueDate {
            let (mut dated, undated): (Vec<_>, Vec<_>) =
                cards.into_iter().partition(|card| card.due_at.is_some());
            dated.sort_by_cached_key(|card| card.due_at.unwrap_or_default());
            if dir == SortDir::Descending {
                dated.reverse();
            }
            dated.extend(undated);
            return dated;
        }
        match self {
            Sort::LastEdited => cards.sort_by_cached_key(|card| card.updated_at),
            Sort::Created => cards.sort_by_cached_key(|card| card.created_at),
            Sort::FirstSide => cards.sort_by_cached_key(|card| folded(&card.preview)),
            Sort::Reviews => cards.sort_by_cached_key(|card| card.reviews),
            Sort::DueDate => unreachable!("handled above, where the undated cards are moved"),
        }
        if dir == SortDir::Descending {
            cards.reverse();
        }
        cards
    }
}

/// Which way an order runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    Ascending,
    Descending,
}

impl SortDir {
    pub fn as_str(self) -> &'static str {
        match self {
            SortDir::Ascending => "asc",
            SortDir::Descending => "desc",
        }
    }

    pub fn parse(text: &str) -> Option<SortDir> {
        match text {
            "asc" => Some(SortDir::Ascending),
            "desc" => Some(SortDir::Descending),
            _ => None,
        }
    }

    pub fn flipped(self) -> SortDir {
        match self {
            SortDir::Ascending => SortDir::Descending,
            SortDir::Descending => SortDir::Ascending,
        }
    }
}

/// A shuffle: an order for a moment, never one a view keeps.
///
/// Seeded so two lists built from the same seed a test apart come out the same, and so a redraw
/// does not reorder the cards under the pointer.
pub fn shuffled(cards: Vec<&CardSummary>, seed: u64) -> Vec<&CardSummary> {
    let mut cards = cards;
    // xorshift64*, which is enough for a shuffle and needs no dependency.
    let mut state = seed | 1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for index in (1..cards.len()).rev() {
        let swap = (next() % (index as u64 + 1)) as usize;
        cards.swap(index, swap);
    }
    cards
}

/// Everything a list is filtered and ordered by.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewSpec {
    /// `None` is the whole library. An id no deck carries is treated as `None`, so a view whose
    /// deck was deleted shows every card rather than none.
    pub deck: Option<Id>,
    pub query: String,
    /// Every tag must match; an empty set matches everything.
    pub tags: Vec<String>,
    pub mode: CardMode,
    /// Only the cards the user pinned.
    pub pinned_only: bool,
    pub sort: Sort,
    pub dir: SortDir,
}

impl ViewSpec {
    /// The list as this spec selects it: filtered, then ordered.
    pub fn cards<'a>(&self, snapshot: &'a Snapshot) -> Vec<&'a CardSummary> {
        let needle = folded(&self.query);
        let wanted: Vec<String> = self.tags.iter().map(|tag| folded(tag)).collect();
        let scope = self
            .deck
            .filter(|deck| snapshot.decks.iter().any(|own| own.id == *deck));
        let subtree = snapshot.subtree(scope);
        let visible = snapshot.studyable_decks();
        let cards: Vec<&CardSummary> = snapshot
            .cards
            .iter()
            .filter(|card| card.mode_matches(self.mode, &visible))
            .filter(|card| scope.is_none() || subtree.contains(&card.deck))
            .filter(|card| !self.pinned_only || card.pinned)
            .filter(|card| {
                wanted.iter().all(|tag| {
                    card.tags
                        .iter()
                        .any(|own| folded(own).as_str() == tag.as_str())
                })
            })
            .filter(|card| {
                needle.is_empty()
                    || card.search_text.contains(&needle)
                    || card.tags.iter().any(|own| folded(own).contains(&needle))
            })
            .collect();
        self.sort.order(cards, self.dir)
    }
}

/// A view as the store holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedView {
    pub id: Id,
    pub name: String,
    pub spec: ViewSpec,
}

impl SavedView {
    /// Whether the deck this view was saved on is gone, which the sidebar says out loud: the view
    /// then shows the whole library rather than an empty list.
    pub fn scope_missing(&self, snapshot: &Snapshot) -> bool {
        self.spec
            .deck
            .is_some_and(|deck| !snapshot.decks.iter().any(|own| own.id == deck))
    }
}

#[cfg(test)]
mod tests;

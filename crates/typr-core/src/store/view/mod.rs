//! Saved filters and the orders a list can be shown in.
//!
//! One type, [`ViewSpec`], is what the sidebar, a saved view and a test all build: a deck scope, a
//! search text, a set of tags, a mode, a pinned-only flag and an order.

use crate::model::{CardSummary, SavedView, Snapshot, Sort, SortDir, ViewSpec, folded};
use crate::shuffle::Shuffle;

impl Sort {
    /// Every order the control offers, in the order it offers them.
    pub const ALL: [Sort; 5] = [
        Sort::LastEdited,
        Sort::Created,
        Sort::FirstSide,
        Sort::DueDate,
        Sort::Reviews,
    ];

    /// The direction this order runs in when it is chosen.
    pub fn natural(self) -> SortDir {
        match self {
            Sort::LastEdited | Sort::Created | Sort::Reviews => SortDir::Descending,
            Sort::FirstSide | Sort::DueDate => SortDir::Ascending,
        }
    }

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

    /// `None` for a key no build of typr writes.
    pub fn parse(text: &str) -> Option<Sort> {
        Sort::ALL.into_iter().find(|sort| sort.as_str() == text)
    }

    /// The cards in this order.
    pub fn order<'a>(&self, cards: Vec<&'a CardSummary>, dir: SortDir) -> Vec<&'a CardSummary> {
        let mut undated = Vec::new();
        let mut keyed: Vec<(Key, &'a CardSummary)> = Vec::with_capacity(cards.len());
        for card in cards {
            match self.key(card) {
                Some(key) => keyed.push((key, card)),
                None => undated.push(card),
            }
        }
        keyed.sort_by(|(left_key, left), (right_key, right)| {
            let order = left_key.cmp(right_key);
            let order = if dir == SortDir::Descending {
                order.reverse()
            } else {
                order
            };
            order.then_with(|| left.id.cmp(&right.id))
        });
        let mut ordered: Vec<&'a CardSummary> = keyed.into_iter().map(|(_, card)| card).collect();
        ordered.extend(undated);
        ordered
    }

    fn key(&self, card: &CardSummary) -> Option<Key> {
        match self {
            Sort::LastEdited => Some(Key::Int(card.updated_at)),
            Sort::Created => Some(Key::Int(card.created_at)),
            Sort::FirstSide => Some(Key::Text(folded(&card.preview))),
            Sort::Reviews => Some(Key::Count(card.reviews)),
            Sort::DueDate => card.due_at.map(Key::Int),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Int(i64),
    Count(u32),
    Text(String),
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
pub fn shuffled(mut cards: Vec<&CardSummary>, seed: u64) -> Vec<&CardSummary> {
    Shuffle::new(seed).reorder(&mut cards);
    cards
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
                    card.tags.iter().any(|own| {
                        let own = folded(own);
                        own.as_str() == tag.as_str() || own.starts_with(&format!("{tag}/"))
                    })
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

impl Snapshot {
    /// The cards a library list shows for a filter: those `spec` keeps, in its order.
    pub fn cards_matching(&self, spec: &ViewSpec) -> Vec<&CardSummary> {
        spec.cards(self)
    }
}

impl SavedView {
    /// Whether the deck this view was saved on is gone.
    pub fn scope_missing(&self, snapshot: &Snapshot) -> bool {
        self.spec
            .deck
            .is_some_and(|deck| !snapshot.decks.iter().any(|own| own.id == deck))
    }
}

#[cfg(test)]
mod tests;

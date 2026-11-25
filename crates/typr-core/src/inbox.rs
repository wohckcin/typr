//! The Inbox: the studyable cards that are waiting, because a prompt is due or was never scheduled.
//!
//! One rule serves the sidebar's counts and the grid the rows open, so a row's number and the cards
//! behind it cannot disagree.

use std::collections::{HashMap, HashSet};

use crate::edits::deck_of_key;
use crate::model::{CardSummary, Id, Snapshot};

/// Which part of the Inbox a selection reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inbox {
    /// Due now or new.
    All,
    /// A prompt is due now.
    Due,
    /// A prompt was never scheduled.
    New,
}

impl Inbox {
    /// The inbox a sidebar key names, if it names one.
    pub fn of_key(key: &str) -> Option<Inbox> {
        match key {
            "inbox" => Some(Inbox::All),
            "inbox:due" => Some(Inbox::Due),
            "inbox:new" => Some(Inbox::New),
            _ => None,
        }
    }
}

/// The decks that are archived or sit under an archived deck. A parent chain that loops is cut at 64
/// hops so a bad snapshot cannot hang the frame.
fn shut_decks(snapshot: &Snapshot) -> HashSet<Id> {
    let archived: HashSet<Id> = snapshot
        .decks
        .iter()
        .filter(|deck| deck.archived)
        .map(|deck| deck.id)
        .collect();
    snapshot
        .decks
        .iter()
        .filter(|deck| {
            let mut current = Some(deck.id);
            for _ in 0..64 {
                let Some(id) = current else { return false };
                if archived.contains(&id) {
                    return true;
                }
                current = snapshot
                    .decks
                    .iter()
                    .find(|d| d.id == id)
                    .and_then(|d| d.parent);
            }
            false
        })
        .map(|deck| deck.id)
        .collect()
}

/// The studyable cards `which` names at `now_ms`, each once: not archived, not trashed, in no
/// archived deck, with at least one prompt (a note never queues).
pub fn inbox_cards(snapshot: &Snapshot, now_ms: i64, which: Inbox) -> Vec<CardSummary> {
    let shut = shut_decks(snapshot);
    snapshot
        .cards
        .iter()
        .filter(|card| {
            !card.archived
                && card.trashed_at.is_none()
                && card.prompts > 0
                && !shut.contains(&card.deck)
        })
        .filter(|card| {
            let due = card.due_at.is_some_and(|at| at <= now_ms);
            let new = card.unscheduled > 0;
            match which {
                Inbox::All => due || new,
                Inbox::Due => due,
                Inbox::New => new,
            }
        })
        .cloned()
        .collect()
}

/// Which cards a sidebar selection shows: the Inbox rows' cards, the cards of one deck
/// `deck:<id>`, or every live card for any other key.
pub fn cards_in(snapshot: &Snapshot, selection: Option<&str>, now_ms: i64) -> Vec<CardSummary> {
    if let Some(which) = selection.and_then(Inbox::of_key) {
        return inbox_cards(snapshot, now_ms, which);
    }
    let deck = selection.and_then(deck_of_key);
    snapshot
        .cards
        .iter()
        .filter(|card| !card.archived && card.trashed_at.is_none())
        .filter(|card| deck.is_none_or(|d| card.deck == d))
        .cloned()
        .collect()
}

/// How many prompts are due in each deck's subtree at `now_ms`: a due card counts for its own deck
/// and every deck above it. Archived decks, archived cards and trashed cards count for nothing.
pub fn due_counts(snapshot: &Snapshot, now_ms: i64) -> HashMap<Id, u32> {
    let parents: HashMap<Id, Option<Id>> = snapshot
        .decks
        .iter()
        .map(|deck| (deck.id, deck.parent))
        .collect();
    let archived: HashSet<Id> = snapshot
        .decks
        .iter()
        .filter(|deck| deck.archived)
        .map(|deck| deck.id)
        .collect();
    let mut counts: HashMap<Id, u32> = HashMap::new();
    for card in &snapshot.cards {
        let Some(due_at) = card.due_at else {
            continue;
        };
        if due_at > now_ms || card.archived || card.trashed_at.is_some() {
            continue;
        }
        let mut current = Some(card.deck);
        // A parent chain that loops would be walked forever; the hop limit keeps a bad snapshot
        // from hanging the frame.
        let mut hops = 0;
        while let Some(id) = current {
            if hops > 64 || archived.contains(&id) {
                break;
            }
            let Some(parent) = parents.get(&id).copied() else {
                break;
            };
            *counts.entry(id).or_insert(0) += 1;
            current = parent;
            hops += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library_view::ViewMode;
    use crate::model::{CardSummary, Deck, Snapshot};

    fn deck(id: Id, parent: Option<Id>, archived: bool) -> Deck {
        Deck {
            id,
            parent,
            name: format!("deck-{id}"),
            archived,
            template: None,
            reverse: None,
            view: ViewMode::Grid,
        }
    }

    fn card(id: Id, deck: Id, prompts: u32, due_at: Option<i64>, unscheduled: u32) -> CardSummary {
        CardSummary {
            id,
            deck,
            prompts,
            due_at,
            unscheduled,
            ..CardSummary::default()
        }
    }

    #[test]
    fn of_key_maps_known_inbox_rows() {
        assert_eq!(Inbox::of_key("inbox"), Some(Inbox::All));
        assert_eq!(Inbox::of_key("inbox:due"), Some(Inbox::Due));
        assert_eq!(Inbox::of_key("inbox:new"), Some(Inbox::New));
        assert_eq!(Inbox::of_key("deck:1"), None);
    }

    #[test]
    fn splits_due_new_and_all_while_filtering_unstudyable_cards() {
        let now = 1_000;
        let mut archived_card = card(4, 1, 1, Some(500), 0);
        archived_card.archived = true;
        let mut trashed_card = card(5, 1, 1, Some(500), 0);
        trashed_card.trashed_at = Some(900);

        let snapshot = Snapshot {
            decks: vec![deck(1, None, false)],
            cards: vec![
                card(1, 1, 1, Some(900), 0),  // due
                card(2, 1, 1, None, 1),       // new
                card(3, 1, 1, Some(2000), 0), // future
                archived_card,
                trashed_card,
                card(6, 1, 0, Some(500), 1), // note (0 prompts)
            ],
            ..Snapshot::default()
        };

        let ids = |which| {
            inbox_cards(&snapshot, now, which)
                .into_iter()
                .map(|c| c.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Inbox::All), vec![1, 2]);
        assert_eq!(ids(Inbox::Due), vec![1]);
        assert_eq!(ids(Inbox::New), vec![2]);
    }

    #[test]
    fn excludes_cards_in_archived_decks_and_descendants() {
        let snapshot = Snapshot {
            decks: vec![
                deck(1, None, true),
                deck(2, Some(1), false),
                deck(3, None, false),
            ],
            cards: vec![
                card(10, 1, 1, Some(10), 0),
                card(20, 2, 1, Some(10), 0),
                card(30, 3, 1, Some(10), 0),
            ],
            ..Snapshot::default()
        };

        let ids: Vec<Id> = inbox_cards(&snapshot, 100, Inbox::All)
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec![30]);
    }

    #[test]
    fn parent_cycle_in_decks_terminates_via_hop_guard() {
        let snapshot = Snapshot {
            decks: vec![deck(1, Some(2), false), deck(2, Some(1), false)],
            cards: vec![card(10, 1, 1, Some(10), 0)],
            ..Snapshot::default()
        };

        let ids: Vec<Id> = inbox_cards(&snapshot, 100, Inbox::All)
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec![10]);
    }

    #[test]
    fn due_counts_rolls_up_subtrees_and_stops_at_archived_or_cyclic_ancestors() {
        let snapshot = Snapshot {
            decks: vec![
                deck(1, None, false),
                deck(2, Some(1), false),
                deck(3, None, true),
                deck(4, Some(3), false),
            ],
            cards: vec![
                card(10, 2, 1, Some(50), 0),  // due in child of 1
                card(11, 1, 1, Some(100), 0), // due in 1
                card(12, 2, 1, Some(200), 0), // future
                card(13, 4, 1, Some(50), 0),  // child of archived 3
            ],
            ..Snapshot::default()
        };
        let counts = due_counts(&snapshot, 100);
        assert_eq!(counts.get(&2).copied(), Some(1));
        assert_eq!(counts.get(&1).copied(), Some(2));
        assert_eq!(counts.get(&3).copied(), None);
        assert_eq!(counts.get(&4).copied(), Some(1));
    }

    #[test]
    fn cards_in_selects_inbox_deck_or_all_live_cards() {
        let snapshot = Snapshot {
            decks: vec![deck(1, None, false), deck(2, None, false)],
            cards: vec![card(10, 1, 1, Some(50), 0), card(20, 2, 0, None, 0)],
            ..Snapshot::default()
        };
        assert_eq!(cards_in(&snapshot, Some("inbox:due"), 100).len(), 1);
        assert_eq!(cards_in(&snapshot, Some("deck:2"), 100)[0].id, 20);
        assert_eq!(cards_in(&snapshot, None, 100).len(), 2);
    }
}

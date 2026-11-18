//! The Inbox: the studyable cards that are waiting, because a prompt is due or was never scheduled.
//!
//! One rule serves the sidebar's counts and the grid the rows open, so a row's number and the cards
//! behind it cannot disagree.

use std::collections::HashSet;

use crate::store::{CardSummary, Id, Snapshot};

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

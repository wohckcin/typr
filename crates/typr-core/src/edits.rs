//! The rules behind card and deck edits, as plain functions, so any front end gets the same
//! answers and they test without a database or a toolkit.

use crate::model::{Deck, Id, Snapshot};

/// Whether the text holds anything once trimmed; an empty card is not saved.
pub fn has_content(markdown: &str) -> bool {
    !markdown.trim().is_empty()
}

/// The deck a sidebar key `deck:<id>` names, if it names one.
pub fn deck_of_key(key: &str) -> Option<Id> {
    key.strip_prefix("deck:")?.parse().ok()
}

/// The deck a new card goes to: the selected deck when it is open, otherwise the first deck that is
/// not archived. `None` when every deck is archived.
pub fn deck_for_new_card(snapshot: &Snapshot, selected_key: Option<&str>) -> Option<Id> {
    let open = |deck: &&Deck| !deck.archived;
    selected_key
        .and_then(deck_of_key)
        .and_then(|id| snapshot.decks.iter().find(|deck| deck.id == id))
        .filter(open)
        .or_else(|| snapshot.decks.iter().find(open))
        .map(|deck| deck.id)
}

/// The card to select once `trashed` has left the list: the one that took its place, or the new
/// last card. `before` and `after` are the shown ids around the trash.
pub fn selection_after_trash(before: &[Id], trashed: Id, after: &[Id]) -> Option<Id> {
    let at = before.iter().position(|id| *id == trashed)?;
    after.get(at).or_else(|| after.last()).copied()
}

/// A deck name without surrounding whitespace, or `None` when nothing is left.
pub fn deck_name(raw: &str) -> Option<&str> {
    Some(raw.trim()).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library_view::ViewMode;
    use crate::model::Deck;

    fn deck(id: Id, archived: bool) -> Deck {
        Deck {
            id,
            parent: None,
            name: format!("deck {id}"),
            archived,
            reverse: None,
            template: None,
            view: ViewMode::default(),
        }
    }

    fn library(decks: &[(Id, bool)]) -> Snapshot {
        Snapshot {
            decks: decks.iter().map(|(id, shut)| deck(*id, *shut)).collect(),
            ..Snapshot::default()
        }
    }

    #[test]
    fn the_selected_open_deck_takes_the_card() {
        let snapshot = library(&[(1, false), (2, false)]);
        assert_eq!(deck_for_new_card(&snapshot, Some("deck:2")), Some(2));
    }

    #[test]
    fn a_selected_archived_deck_falls_back() {
        let snapshot = library(&[(1, false), (2, true)]);
        assert_eq!(deck_for_new_card(&snapshot, Some("deck:2")), Some(1));
    }

    #[test]
    fn an_archived_first_deck_is_skipped() {
        let snapshot = library(&[(1, true), (2, false)]);
        assert_eq!(deck_for_new_card(&snapshot, Some("inbox")), Some(2));
        assert_eq!(deck_for_new_card(&snapshot, None), Some(2));
    }

    #[test]
    fn no_open_deck_gives_none() {
        let snapshot = library(&[(1, true), (2, true)]);
        assert_eq!(deck_for_new_card(&snapshot, None), None);
        assert_eq!(deck_for_new_card(&library(&[]), Some("deck:1")), None);
    }

    #[test]
    fn a_trash_selects_the_neighbour_then_the_last_then_nothing() {
        assert_eq!(selection_after_trash(&[1, 2, 3], 2, &[1, 3]), Some(3));
        assert_eq!(selection_after_trash(&[1, 2, 3], 3, &[1, 2]), Some(2));
        assert_eq!(selection_after_trash(&[1], 1, &[]), None);
        assert_eq!(selection_after_trash(&[1, 2], 9, &[1, 2]), None);
    }

    #[test]
    fn a_name_is_trimmed_and_a_blank_one_is_refused() {
        assert_eq!(deck_name("  Spanish "), Some("Spanish"));
        assert_eq!(deck_name("  \t"), None);
    }

    #[test]
    fn a_whitespace_card_has_no_content() {
        assert!(has_content("Question"));
        assert!(!has_content("  \n\t "));
    }

    #[test]
    fn a_key_names_a_deck_only_as_deck_colon_id() {
        assert_eq!(deck_of_key("deck:7"), Some(7));
        assert_eq!(deck_of_key("deck:x"), None);
        assert_eq!(deck_of_key("inbox"), None);
    }
}

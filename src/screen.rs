//! The window's screens and the transitions between them.
//!
//! One `enum Screen` whose variants carry only the data valid on them, and one `step` that turns
//! an event into effects instead of performing them. Nothing here touches the window, the store
//! or the clock, which is what lets a test drive every path.

use rune::machine::{Effects, Seq};

/// A card's identity in the store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardId(pub i64);

/// Which queue a session works through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Queue {
    New,
    Due,
}

/// What the window is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Library,
    Editor { card: Option<CardId> },
    Session { queue: Queue },
    Summary,
}

/// A screen-level event: the actions that move between screens, and the queue running out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    OpenCard(CardId),
    NewCard,
    CloseEditor,
    StartSession(Queue),
    QueueEmpty,
    Leave,
}

/// Work the caller carries out after a transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Put this card's text in the editor.
    LoadCard(CardId),
}

/// Move one screen on, given one event.
pub fn step(seq: &mut Seq, state: &mut Screen, event: Event) -> Effects<Effect> {
    let _ = seq;
    match (&*state, event) {
        (Screen::Library, Event::OpenCard(card)) => {
            *state = Screen::Editor { card: Some(card) };
            Effects::one(Effect::LoadCard(card))
        }
        (Screen::Library, Event::NewCard) => {
            *state = Screen::Editor { card: None };
            Effects::none()
        }
        (Screen::Editor { .. }, Event::CloseEditor) => {
            *state = Screen::Library;
            Effects::none()
        }
        (Screen::Library, Event::StartSession(queue)) => {
            *state = Screen::Session { queue };
            Effects::none()
        }
        (Screen::Session { .. }, Event::QueueEmpty) => {
            *state = Screen::Summary;
            Effects::none()
        }
        (Screen::Session { .. } | Screen::Summary, Event::Leave) => {
            *state = Screen::Library;
            Effects::none()
        }
        _ => Effects::none(),
    }
}

#[cfg(test)]
mod tests;

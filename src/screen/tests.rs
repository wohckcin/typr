use rune::machine::Seq;

use super::{CardId, Effect, Event, Queue, Screen, step};

fn drive(from: Screen, event: Event) -> (Screen, Vec<Effect>) {
    let mut seq = Seq::default();
    let mut screen = from;
    let effects = step(&mut seq, &mut screen, event);
    (screen, effects.into_iter().collect())
}

#[test]
fn opening_a_card_loads_it() {
    let (screen, effects) = drive(Screen::Library, Event::OpenCard(CardId(7)));
    assert_eq!(
        screen,
        Screen::Editor {
            card: Some(CardId(7))
        }
    );
    assert_eq!(effects, vec![Effect::LoadCard(CardId(7))]);
}

#[test]
fn a_new_card_opens_an_empty_editor() {
    let (screen, effects) = drive(Screen::Library, Event::NewCard);
    assert_eq!(screen, Screen::Editor { card: None });
    assert!(effects.is_empty());
}

#[test]
fn closing_the_editor_returns_to_the_library() {
    let (screen, _) = drive(
        Screen::Editor {
            card: Some(CardId(1)),
        },
        Event::CloseEditor,
    );
    assert_eq!(screen, Screen::Library);
}

#[test]
fn a_session_runs_its_queue_to_the_summary() {
    let (screen, _) = drive(Screen::Library, Event::StartSession(Queue::Due));
    assert_eq!(screen, Screen::Session { queue: Queue::Due });

    let (screen, _) = drive(screen, Event::QueueEmpty);
    assert_eq!(screen, Screen::Summary);

    let (screen, _) = drive(screen, Event::Leave);
    assert_eq!(screen, Screen::Library);
}

#[test]
fn leaving_a_session_mid_queue_returns_to_the_library() {
    let (screen, _) = drive(Screen::Session { queue: Queue::New }, Event::Leave);
    assert_eq!(screen, Screen::Library);
}

#[test]
fn an_event_that_does_not_apply_changes_nothing() {
    let (screen, effects) = drive(Screen::Summary, Event::OpenCard(CardId(3)));
    assert_eq!(screen, Screen::Summary);
    assert!(effects.is_empty());
}

use super::*;

fn session_state() -> State {
    State {
        screen: Screen::Session,
        session: Session::new(
            Queue::Due,
            vec![Entry::sides(
                7,
                ["What is a leap year?", "A year with 366 days."],
                0,
            )],
        ),
        ..State::default()
    }
}

fn key_event(key: rune::Key) -> rune::KeyEvent {
    rune::KeyEvent {
        key,
        mods: rune::Mods::default(),
    }
}

#[test]
fn settings_open_from_a_session_and_return_to_the_card() {
    let event = key_event(rune::Key::Text(",".to_string()));
    let chord = crate::keymap::from_event(&event);
    assert_eq!(
        chord,
        Some(typr_core::keymap::Chord::key(typr_core::keymap::Key::Char(
            ','
        ))),
        "chord"
    );
    assert_eq!(
        typr_core::keymap::dispatch(Surface::Session, chord.expect("chord")),
        Some(Command::Settings),
        "dispatch"
    );

    let mut state = session_state();
    let card = state.session.current().map(|entry| entry.card);
    assert_eq!(card, Some(7), "the session starts on the card it was given");
    assert!(
        crate::on_key(&mut state, &event, &mut Tx::default()),
        "on_key took the comma"
    );
    assert_eq!(
        state.screen,
        Screen::Settings,
        "the settings key opens the screen from a session"
    );
    assert_eq!(
        state.session.current().map(|entry| entry.card),
        card,
        "opening settings leaves the session on the same card"
    );

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("settings-title").is_some(),
        "and the screen is drawn"
    );

    assert_eq!(
        typr_core::keymap::dispatch(
            Surface::Settings,
            typr_core::keymap::Chord::key(typr_core::keymap::Key::Escape)
        ),
        Some(Command::Leave),
        "the settings screen has a chord to leave"
    );
    assert!(
        crate::on_key(
            &mut runtime.state,
            &key_event(rune::Key::Escape),
            &mut Tx::default()
        ),
        "on_key took the escape"
    );
    assert_eq!(
        runtime.state.screen,
        Screen::Session,
        "leaving settings returns to the session that opened it"
    );
    assert_eq!(
        runtime.state.session.current().map(|entry| entry.card),
        card,
        "with the card that was up still up"
    );
}

#[test]
fn settings_open_from_the_library_and_return_to_it() {
    let event = key_event(rune::Key::Text(",".to_string()));
    let chord = crate::keymap::from_event(&event).expect("chord");
    assert_eq!(
        typr_core::keymap::dispatch(Surface::Library, chord),
        Some(Command::Settings),
        "the library has a chord to open settings"
    );

    let mut state = State::default();
    assert!(
        crate::on_key(&mut state, &event, &mut Tx::default()),
        "on_key took the comma"
    );
    assert_eq!(state.screen, Screen::Settings);

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("settings-title").is_some(),
        "and the screen is drawn"
    );
    assert!(
        crate::on_key(
            &mut runtime.state,
            &key_event(rune::Key::Escape),
            &mut Tx::default()
        ),
        "on_key took the escape"
    );
    assert_eq!(runtime.state.screen, Screen::Library);
}

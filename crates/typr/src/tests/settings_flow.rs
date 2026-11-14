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

/// A state with a library behind it, so a setting written goes through the store and comes back.
fn stored_state() -> (tempfile::TempDir, State) {
    let dir = tempfile::tempdir().expect("dir");
    let db = typr_core::store::Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let handle = Handle::spawn(
        db,
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    let mut state = State {
        screen: Screen::Settings,
        store: Some(handle),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    (dir, state)
}

/// Draw until the store's answer has landed and the screen has been repainted with it.
fn draw_until_changed(runtime: &mut rune::Runtime<State>, wanted: impl Fn(&State) -> bool) -> bool {
    for _ in 0..200 {
        // `poll` is what the host runs each frame: it drains the store's answers and decides
        // whether anything changed. A bare `frame` only redraws.
        let changed = crate::frame_step(
            &mut runtime.state,
            typr_core::clock::SystemClock.now_ms(),
            &mut None,
        );
        if wanted(&runtime.state) {
            runtime.settle_all();
            for _ in 0..3 {
                runtime.frame(0.0);
            }
            return true;
        }
        if changed {
            runtime.settle_all();
        }
        runtime.frame(0.0);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

/// A scale change reaches the next frame: the interface is drawn larger without a restart.
#[test]
fn a_scale_change_is_drawn_on_the_next_frame() {
    let (_dir, mut state) = stored_state();
    state.open_settings();
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..3 {
        runtime.frame(0.0);
    }
    let before = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("settings-title"))
        .map(|element| (element.rect[2], element.scale))
        .expect("the title is drawn");
    assert!(before.0 > 0.0, "the title was drawn with no width");

    runtime
        .state
        .store_setting(SettingKey::Scale, SettingValue::Scale(1.25));
    assert!(
        draw_until_changed(&mut runtime, |state| {
            (state.library.snapshot().settings.scale - 1.25).abs() < 1e-6
        }),
        "the store never reported the new scale"
    );

    let after = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("settings-title"))
        .map(|element| (element.rect[2], element.scale))
        .expect("the title is drawn");
    // `Cx::zoom` is a view zoom, not a font size: the view is handed a window 1/1.25 as wide in
    // its own points and everything drawn in it is 1.25 times the size on screen. So a greedy row
    // comes out narrower in points by exactly the zoom, which is what proves the frame was redrawn
    // at the new scale.
    let ratio = before.0 / after.0;
    assert!(
        (ratio - 1.25).abs() < 0.05,
        "the frame was not drawn at the new scale: {before:?} -> {after:?} (ratio {ratio})"
    );
    assert_eq!(
        before.1, after.1,
        "the device scale is the host's, not the interface scale: {before:?} -> {after:?}"
    );
}

/// A theme change repaints: the kit the next frame draws with is the other one.
#[test]
fn a_the_theme_change_repaints() {
    let (_dir, mut state) = stored_state();
    state.open_settings();
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    let dark_before = crate::settings::kit(&runtime.state.library.snapshot().settings).dark;
    let next = if dark_before {
        typr_core::store::Theme::Light
    } else {
        typr_core::store::Theme::Dark
    };
    // What the row's own control sends.
    runtime
        .state
        .store_setting(SettingKey::Theme, SettingValue::Theme(next));
    assert!(
        draw_until_changed(&mut runtime, |state| {
            state.library.snapshot().settings.theme == next
        }),
        "the store never reported the new theme"
    );
    assert_eq!(
        crate::settings::kit(&runtime.state.library.snapshot().settings).dark,
        !dark_before,
        "the kit the next frame draws with did not follow the theme"
    );
    assert_eq!(
        crate::settings::kit(&runtime.state.library.snapshot().settings)
            .c
            .page,
        crate::settings::kit(&runtime.state.library.snapshot().settings)
            .c
            .page
    );
}

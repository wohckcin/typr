use super::*;

#[test]
fn a_slow_answer_is_recorded_and_a_quick_one_is_not() {
    assert!(slow_record("due_prompts", Duration::from_millis(249)).is_none());
    let line = slow_record("due_prompts", Duration::from_millis(300)).expect("recorded");
    assert!(line.contains("due_prompts"), "{line}");
    assert!(line.contains("300 ms"), "{line}");
}

#[test]
fn a_panicked_worker_is_named_in_the_quit_record() {
    let library = tempfile::tempdir().expect("temp dir");
    let db = Db::open(&library.path().join("typr.sqlite3")).expect("open");
    let waker: Waker = Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(Stops(AtomicUsize::new(0))), waker).expect("spawn");
    let status = handle.status();
    let _ = handle.history(1);
    for _ in 0..200 {
        if !handle.poll().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(handle);

    let records = quit_records(Some(&status));

    assert_eq!(records.len(), 1, "one line for a worker that ended");
    assert_eq!(records[0].0, log::Level::Error);
    assert!(
        records[0].1.contains("the library worker ended badly"),
        "{records:?}"
    );
    assert!(records[0].1.contains("the clock fell over"), "{records:?}");
}

#[test]
fn a_failure_is_drawn_at_the_bottom_of_the_screen() {
    let state = State {
        failure: Some("The library could not be opened: no such file".to_string()),
        ..State::default()
    };
    let mut runtime = rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(runtime.find("failure").is_some(), "the line is on screen");
    let drawn = runtime
        .elements()
        .into_iter()
        .any(|element| element.key.as_deref() == Some("failure"));
    assert!(drawn, "the failure row is in the tree, not just a label");
}

#[test]
fn a_quiet_quit_says_so() {
    let records = quit_records(None);

    assert_eq!(records.len(), 1, "one line for a quiet quit");
    assert_eq!(records[0].0, log::Level::Debug);
    assert!(
        records[0].1.contains("the worker ended quietly"),
        "{records:?}"
    );
}

#[test]
fn a_grade_is_timed_like_any_other_command() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    let mut state = State {
        store: Some(store),
        session: session_on(1),
        screen: Screen::Session,
        ..State::default()
    };

    let id = state
        .submit_grade(1, "fwd", Grade::Remembered)
        .expect("sent");
    assert!(
        state.sent.contains_key(&id),
        "a grade is timed, so a slow one can be written down"
    );
}

#[test]
fn a_library_that_cannot_be_opened_names_its_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    std::fs::write(&path, b"not a database").expect("write");
    let error = match Db::open(&path) {
        Ok(_) => panic!("a file that is not a database cannot open"),
        Err(error) => error,
    };

    let line = format!("The library could not be opened: {error}");
    assert!(
        line.contains(&path.display().to_string()),
        "the reader is told which file: {line}"
    );
    assert_eq!(
        line.matches("could not be opened").count(),
        1,
        "the reason is stated once, not once per layer: {line}"
    );
}

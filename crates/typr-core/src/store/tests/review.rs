//! Scheduling: what a record does to a prompt, the queues, the cap and the day's counts.

use super::*;

#[test]
fn three_learning_cycles_then_two_reviews_report_a_count_of_two() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Counting");
    let card = card(&mut db, deck, "front\n---\nback");

    let mut at = 1_700_000_000_000;
    for _ in 0..3 {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Forgot,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 10 * 60_000;
    }
    for _ in 0..2 {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 24 * 60 * 60_000;
    }

    let history = db.history(card).expect("history");
    assert_eq!(history.len(), 5);
    assert_eq!(
        history.iter().map(|row| row.phase).collect::<Vec<_>>(),
        vec![
            Phase::Learn,
            Phase::Learn,
            Phase::Learn,
            Phase::Review,
            Phase::Review
        ]
    );
    assert_eq!(db.review_count(card).expect("count"), 2);
}

#[test]
fn a_prompt_always_equals_the_fold_of_its_records() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Fold");

    // Five prompts, each pushed into a different phase by a different grade sequence.
    let sequences: [&[Grade]; 5] = [
        &[Grade::Remembered],
        &[Grade::Forgot],
        &[Grade::Remembered, Grade::Remembered],
        &[Grade::Remembered, Grade::Remembered, Grade::Remembered],
        &[Grade::Remembered, Grade::Remembered, Grade::Forgot],
    ];
    let mut cards = Vec::new();
    for (index, sequence) in sequences.iter().enumerate() {
        let card = card(&mut db, deck, &format!("front {index}\n---\nback {index}"));
        let mut at = 1_700_000_000_000;
        for grade in sequence.iter() {
            db.apply(
                &Command::Grade {
                    card,
                    key: "fwd".to_string(),
                    grade: *grade,
                    duration_ms: None,
                },
                at,
            )
            .expect("grade");
            at += 11 * 60_000;
        }
        cards.push(card);
    }

    let phases: Vec<Phase> = cards
        .iter()
        .map(|card| {
            db.prompt(*card, "fwd")
                .expect("prompt")
                .expect("exists")
                .phase
        })
        .collect();
    assert!(
        phases.contains(&Phase::Learn)
            && phases.contains(&Phase::Relearn)
            && phases.contains(&Phase::Review)
    );

    for card in &cards {
        let stored = db.prompt(*card, "fwd").expect("prompt");
        let folded = db.fold_records(*card, "fwd").expect("fold");
        assert_eq!(stored, folded, "card {card} disagrees with its history");
    }

    // Rebuilding from history changes nothing, which is the invariant in action.
    assert_eq!(db.rebuild_prompts().expect("rebuild"), cards.len());
    for card in &cards {
        assert_eq!(
            db.prompt(*card, "fwd").expect("prompt"),
            db.fold_records(*card, "fwd").expect("fold")
        );
    }
}

#[test]
fn prompt_keys_beyond_fwd_fold_into_their_own_prompts() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Keys");
    let card = card(&mut db, deck, "front\n---\nback");

    // Records a later version will write: a reverse direction, a cloze group and a mask group.
    let mut at = 1_700_000_000_000;
    for key in ["clair", "cloze:1", "mask:a1"] {
        db.append_record_for_test(Record {
            card,
            key,
            phase: Phase::Review,
            grade: Grade::Remembered,
            at,
            interval_ms: 86_400_000,
            due_at: Some(at + 86_400_000),
        })
        .expect("record");
        at += 1_000;
    }

    // Only the three keys that have records are rewritten; `fwd` has none and keeps its row, and
    // `rev` came from the card having two sides.
    assert_eq!(db.rebuild_prompts().expect("rebuild"), 3);
    assert_eq!(
        db.prompt_keys(card).expect("keys"),
        vec![
            "clair".to_string(),
            "cloze:1".to_string(),
            "fwd".to_string(),
            "mask:a1".to_string(),
            "rev".to_string()
        ]
    );
    for key in ["clair", "cloze:1", "mask:a1"] {
        assert_eq!(
            db.prompt(card, key).expect("prompt"),
            db.fold_records(card, key).expect("fold")
        );
    }
}

/// Noon-ish of a fixed day, so a test never straddles a rollover.
const NOON: i64 = 1_768_089_600_000 + 13 * 60 * 60 * 1000;
const HOUR: i64 = 60 * 60 * 1000;
const WEEK: i64 = 7 * 24 * HOUR;

/// Put a prompt's schedule where a record would have put it.
#[test]
fn the_most_overdue_study_day_comes_first() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Urgency");

    let week_ago = card(&mut db, deck, "old\n---\nancient");
    let an_hour_ago = card(&mut db, deck, "recent\n---\nnow");
    let two_hours_ago = card(&mut db, deck, "also recent\n---\nnow");
    schedule(&mut db, week_ago, NOON - WEEK);
    schedule(&mut db, an_hour_ago, NOON - HOUR);
    schedule(&mut db, two_hours_ago, NOON - 2 * HOUR);

    let due = db.due_prompts(NOON, 0, 11, None).expect("due");
    assert_eq!(due.len(), 3);
    assert_eq!(due[0].card, week_ago);
}

#[test]
fn prompts_due_on_the_same_study_day_are_ordered_by_the_shuffle() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Ties");
    let first = card(&mut db, deck, "a\n---\nb");
    let second = card(&mut db, deck, "c\n---\nd");
    schedule(&mut db, first, NOON - 2 * HOUR);
    schedule(&mut db, second, NOON - HOUR);

    let mut orders = std::collections::BTreeSet::new();
    for seed in 1..40 {
        let due = db.due_prompts(NOON, 0, seed, None).expect("due");
        orders.insert(due.iter().map(|prompt| prompt.card).collect::<Vec<_>>());
    }
    assert_eq!(
        orders.len(),
        2,
        "the timestamp must not decide the order: {orders:?}"
    );
}

#[test]
fn trashed_archived_and_note_prompts_are_never_due() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let archived_root = deck(&mut db, "Archived root");
    let child = match db.apply(
        &Command::CreateDeck {
            parent: Some(archived_root),
            name: "Child".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let open = deck(&mut db, "Open");

    let kept = card(&mut db, open, "kept\n---\nkept");
    let trashed = card(&mut db, open, "trashed\n---\ntrashed");
    let under_archived = card(&mut db, child, "archived\n---\narchived");
    let note = card(&mut db, open, "just a note");
    for card in [kept, trashed, under_archived, note] {
        schedule(&mut db, card, NOON - HOUR);
    }
    db.apply(&Command::TrashCard { id: trashed }, NOON)
        .expect("trash");
    db.apply(
        &Command::ArchiveDeck {
            id: archived_root,
            archived: true,
        },
        NOON,
    )
    .expect("archive");

    let due = db.due_prompts(NOON, 0, 5, None).expect("due");
    assert_eq!(
        due.iter().map(|prompt| prompt.card).collect::<Vec<_>>(),
        vec![kept]
    );
}

#[test]
fn a_prompt_due_tomorrow_is_not_in_todays_queue() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Tomorrow");
    let card = card(&mut db, deck, "a\n---\nb");
    schedule(&mut db, card, NOON + 24 * HOUR);

    assert!(db.due_prompts(NOON, 0, 1, None).expect("due").is_empty());
    assert_eq!(
        db.due_prompts(NOON + 25 * HOUR, 0, 1, None)
            .expect("due")
            .len(),
        1
    );
}

#[test]
fn resetting_history_makes_a_card_new_again() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Reset");
    let card = card(&mut db, deck, "front\n---\nback");

    let mut at = NOON - WEEK;
    for _ in 0..3 {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 24 * HOUR;
    }
    assert_eq!(db.review_count(card).expect("count"), 3);

    db.apply(
        &Command::ResetHistory {
            card,
            key: "fwd".to_string(),
        },
        NOON,
    )
    .expect("reset");

    assert!(
        db.history(card).expect("history").is_empty(),
        "learn records go too"
    );
    assert_eq!(db.review_count(card).expect("count"), 0);
    assert_eq!(db.fold_records(card, "fwd").expect("fold"), None);
    let prompt = db
        .prompt(card, "fwd")
        .expect("prompt")
        .expect("still exists");
    assert_eq!(
        (prompt.phase, prompt.interval_ms, prompt.due_at, prompt.reps),
        (Phase::Learn, 0, None, 0)
    );
    assert!(
        db.due_prompts(NOON, 0, 1, None).expect("due").is_empty(),
        "the card stops being due"
    );
}

#[test]
fn the_cap_counts_graduated_cards_across_decks_and_ignores_learning_cycles() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let first_deck = deck(&mut db, "First");
    let second_deck = deck(&mut db, "Second");
    let stubborn = card(&mut db, first_deck, "a\n---\nb");
    let other = card(&mut db, second_deck, "c\n---\nd");

    let mut at = NOON;
    for _ in 0..3 {
        db.apply(
            &Command::Grade {
                card: stubborn,
                key: "fwd".to_string(),
                grade: Grade::Forgot,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 60_000;
    }
    assert_eq!(
        db.learned_on(crate::day::study_day(NOON, 0), 0)
            .expect("learned"),
        0
    );

    db.apply(
        &Command::Grade {
            card: stubborn,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        at,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card: other,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        at + 60_000,
    )
    .expect("grade");

    assert_eq!(
        db.learned_on(crate::day::study_day(NOON, 0), 0)
            .expect("learned"),
        2
    );
}

#[test]
fn the_new_queue_stops_at_the_cap_and_resumes_the_next_day() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Backlog");
    for index in 0..3 {
        card(&mut db, deck, &format!("front {index}\n---\nback {index}"));
    }
    db.apply(
        &Command::SetSetting {
            key: "new_per_day".to_string(),
            value: "2".to_string(),
        },
        NOON,
    )
    .expect("setting");
    assert_eq!(db.settings().expect("settings").new_per_day, 2);

    assert_eq!(
        db.new_prompts(NOON, 0, 3, None).expect("new").len(),
        2,
        "the queue stops at the cap"
    );

    // Learning two of them spends today's allowance.
    let waiting = db.new_prompts(NOON, 0, 3, None).expect("new");
    for prompt in waiting {
        db.apply(
            &Command::Grade {
                card: prompt.card,
                key: prompt.key,
                grade: Grade::Remembered,
                duration_ms: None,
            },
            NOON + HOUR,
        )
        .expect("grade");
    }
    assert!(
        db.new_prompts(NOON + HOUR, 0, 3, None)
            .expect("new")
            .is_empty(),
        "the day is spent"
    );

    let tomorrow = NOON + 24 * HOUR;
    assert_eq!(
        db.new_prompts(tomorrow, 0, 3, None).expect("new").len(),
        1,
        "the last card waits for tomorrow"
    );
}

#[test]
fn statistics_are_counted_per_study_day() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Stats");
    let card = card(&mut db, deck, "a\n---\nb");

    let today = crate::day::study_day(NOON, 0);
    let tomorrow_at = NOON + 24 * HOUR;
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON + 600_000,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Forgot,
            duration_ms: None,
        },
        tomorrow_at,
    )
    .expect("grade");

    let today_stats = db.stats_for_day(today, 0).expect("stats");
    assert_eq!(today_stats.reviews, 2);
    assert_eq!(today_stats.lapses, 0);
    let tomorrow_stats = db.stats_for_day(today + 1, 0).expect("stats");
    assert_eq!(tomorrow_stats.reviews, 1);
    assert_eq!(tomorrow_stats.lapses, 1);
}

#[test]
fn a_new_card_forgotten_while_learning_is_still_new() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let subject = deck(&mut db, "Biology");
    let failed = card(&mut db, subject, " mitosis\n---\n cell division");
    db.apply(
        &Command::Grade {
            card: failed,
            key: "fwd".to_string(),
            grade: Grade::Forgot,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");

    let new: Vec<Id> = db
        .new_prompts(NOON, 0, 1, None)
        .expect("new")
        .into_iter()
        .map(|prompt| prompt.card)
        .collect();
    assert_eq!(new, vec![failed], "a failed first lesson is still new");
    assert!(
        db.due_prompts(NOON, 0, 1, None).expect("due").is_empty(),
        "and it is not due, because nothing was scheduled"
    );
}

#[test]
fn a_hidden_text_card_gets_one_prompt_per_group() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let card = card(
        &mut db,
        deck,
        "{{1::Insulin}} is made by the {{2::pancreas}}, and {{glucagon}} opposes it.",
    );

    assert_eq!(
        db.prompt_keys(card).expect("the keys"),
        ["cloze:0", "cloze:1", "cloze:2"],
        "one prompt per group, and the loose span is its own group"
    );
}

#[test]
fn a_card_that_stops_hiding_text_keeps_its_history() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let card = card(&mut db, deck, "The {{hormone}} is made by the pancreas.");

    db.apply(
        &Command::Grade {
            card,
            key: "cloze:0".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");
    assert_eq!(db.review_count(card).expect("count"), 1);

    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "The hormone is made by the pancreas.".to_string(),
        },
        NOON,
    )
    .expect("save");

    assert_eq!(
        db.prompt_keys(card).expect("the keys"),
        ["cloze:0"],
        "the prompt is kept, history and all"
    );
    assert_eq!(
        db.review_count(card).expect("count"),
        1,
        "the record survives"
    );
    let queued = db.new_prompts(NOON, 0, 1, None).expect("the queue");
    assert!(
        queued.iter().all(|prompt| prompt.card != card),
        "a card that no longer hides text is out of the queue"
    );
}

#[test]
fn a_one_line_hidden_text_card_is_in_the_new_queue() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let card = card(&mut db, deck, "The {{hormone}} is made by the pancreas.");

    let queued: Vec<Id> = db
        .new_prompts(NOON, 0, 1, None)
        .expect("the queue")
        .into_iter()
        .map(|prompt| prompt.card)
        .collect();
    assert_eq!(queued, [card], "one line with hidden text is studyable");
}

#[test]
fn two_groups_of_one_card_count_once_against_the_cap() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    db.apply(
        &Command::SetSetting {
            key: "new_per_day".to_string(),
            value: "1".to_string(),
        },
        0,
    )
    .expect("cap to one");
    let deck = deck(&mut db, "Biology");
    card(&mut db, deck, "{{1::Insulin}} and {{2::glucagon}}.");

    let alone = db.new_prompts(NOON, 0, 7, None).expect("the queue");
    let mut keys: Vec<&str> = alone.iter().map(|prompt| prompt.key.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["cloze:1", "cloze:2"],
        "one card's allowance buys both of its groups"
    );

    let second = card(&mut db, deck, "The {{hormone}} matters.");
    for seed in 1..12 {
        let queued = db.new_prompts(NOON, 0, seed, None).expect("the queue");
        let cards: std::collections::HashSet<Id> =
            queued.iter().map(|prompt| prompt.card).collect();
        assert_eq!(
            cards.len(),
            1,
            "the cap counts cards, not prompts: {queued:?}"
        );
        assert!(
            !queued.iter().any(|prompt| prompt.card == second)
                || queued.iter().all(|prompt| prompt.card == second),
            "a card is taken whole: {queued:?}"
        );
    }
}

#[test]
fn a_group_whose_spans_are_gone_stops_queueing() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let under = deck(&mut db, "Biology");
    let card = card(&mut db, under, "{{1::alpha}} {{2::beta}}");
    match db
        .apply(
            &Command::Grade {
                card,
                key: "cloze:2".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            NOON,
        )
        .expect("grade")
    {
        Outcome::Graded { .. } => {}
        other => panic!("expected a grade: {other:?}"),
    }

    // The second group's spans are gone, so its key can no longer be asked: masking it would hide
    // nothing and the review would print the answer as the question.
    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "{{1::alpha}} only".to_string(),
        },
        NOON,
    )
    .expect("save");

    assert_eq!(
        db.prompt_keys(card).expect("keys"),
        vec!["cloze:1".to_string(), "cloze:2".to_string()],
        "a retired prompt keeps its row and its history"
    );
    let retired: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT retired FROM prompt WHERE card_id = ?1 AND key = 'cloze:2'",
            [card],
            |row| row.get(0),
        )
        .expect("the retirement flag");
    assert_eq!(retired, 1, "the key is marked retired");
    assert!(
        db.due_prompts(NOON + 400 * 24 * HOUR, 0, 1, None)
            .expect("due")
            .iter()
            .all(|prompt| prompt.key != "cloze:2"),
        "a retired prompt is never due"
    );

    // Asking for it again clears the mark, so the schedule it kept is used again.
    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "{{1::alpha}} {{2::beta}}".to_string(),
        },
        NOON,
    )
    .expect("save again");
    let retired: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT retired FROM prompt WHERE card_id = ?1 AND key = 'cloze:2'",
            [card],
            |row| row.get(0),
        )
        .expect("the flag after the second save");
    assert_eq!(retired, 0, "a returning group is answerable again");
}

#[test]
fn a_two_sided_card_holds_both_directions() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Directions");
    let card = card(&mut db, deck, "front\n---\nback");

    assert_eq!(
        db.prompt_keys(card).expect("keys"),
        vec!["fwd".to_string(), "rev".to_string()],
        "a card that can be read either way carries a prompt for each reading"
    );
}

#[test]
fn a_note_holds_no_reverse_prompt() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Notes");
    let note = card(&mut db, deck, "just a line");

    assert!(
        db.prompt_keys(note).expect("keys").is_empty(),
        "a note has nothing to ask, in either direction"
    );
}

#[test]
fn saving_a_two_sided_card_leaves_its_reverse_prompt_unretired() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Directions");
    let card = card(&mut db, deck, "front\n---\nback");

    // The save claims every key the card holds; `rev` outside that list would be retired by the same
    // call that keeps `fwd` alive.
    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "front\n---\nback again".to_string(),
        },
        NOON,
    )
    .expect("save");

    assert_eq!(
        retired(&db, card, "rev"),
        0,
        "the reverse prompt is answerable"
    );
    assert_eq!(retired(&db, card, "fwd"), 0, "and so is the forward one");
}

#[test]
fn a_card_that_stops_being_two_sided_retires_its_reverse_prompt() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Directions");
    let card = card(&mut db, deck, "front\n---\nback");

    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "one line now".to_string(),
        },
        NOON,
    )
    .expect("save");

    assert_eq!(retired(&db, card, "rev"), 1, "nothing is left to ask");
    assert_eq!(
        db.prompt_keys(card).expect("keys"),
        vec!["fwd".to_string(), "rev".to_string()],
        "the rows stay, because their history does"
    );
}

/// The retirement mark on one prompt, which is what keeps it out of the queues.
fn retired(db: &Db, card: Id, key: &str) -> i64 {
    db.connection_for_test()
        .query_row(
            "SELECT retired FROM prompt WHERE card_id = ?1 AND key = ?2",
            rusqlite::params![card, key],
            |row| row.get(0),
        )
        .expect("the retirement flag")
}

#[test]
fn a_card_setting_beats_its_deck() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let root = deck(&mut db, "Languages");
    let child = subdeck(&mut db, root, "Japanese");
    let quiet = card(&mut db, child, "front\n---\nback");
    let loud = card(&mut db, child, "front\n---\nback");

    db.apply(
        &Command::SetDeckReverse {
            deck: root,
            reverse: Some(true),
        },
        NOON,
    )
    .expect("deck on");
    assert_eq!(
        reverse_queue(&db),
        vec![quiet, loud],
        "the deck setting reaches its child"
    );

    db.apply(
        &Command::SetCardReverse {
            card: quiet,
            reverse: Some(false),
        },
        NOON,
    )
    .expect("card off");
    assert_eq!(
        reverse_queue(&db),
        vec![loud],
        "the card's own setting wins"
    );

    db.apply(
        &Command::SetCardReverse {
            card: quiet,
            reverse: None,
        },
        NOON,
    )
    .expect("card inherits");
    assert_eq!(
        reverse_queue(&db),
        vec![quiet, loud],
        "clearing the card's value lets the deck speak again"
    );
}

#[test]
fn a_deck_setting_reaches_nested_decks() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let root = deck(&mut db, "Languages");
    let middle = subdeck(&mut db, root, "Japanese");
    let leaf = subdeck(&mut db, middle, "Verbs");
    let card = card(&mut db, leaf, "front\n---\nback");

    db.apply(
        &Command::SetDeckReverse {
            deck: root,
            reverse: Some(true),
        },
        NOON,
    )
    .expect("root on");
    assert_eq!(
        reverse_queue(&db),
        vec![card],
        "two levels down still inherits"
    );

    db.apply(
        &Command::SetDeckReverse {
            deck: middle,
            reverse: Some(false),
        },
        NOON,
    )
    .expect("middle off");
    assert!(reverse_queue(&db).is_empty(), "the nearer deck wins");

    db.apply(
        &Command::SetDeckReverse {
            deck: middle,
            reverse: None,
        },
        NOON,
    )
    .expect("middle inherits");
    assert_eq!(
        reverse_queue(&db),
        vec![card],
        "and clearing it hands back to the root"
    );
}

#[test]
fn the_library_default_is_the_last_word() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let root = deck(&mut db, "Languages");
    let card = card(&mut db, root, "front\n---\nback");

    assert!(reverse_queue(&db).is_empty(), "reverse starts off");
    db.apply(
        &Command::SetSetting {
            key: "reverse_default".to_string(),
            value: "1".to_string(),
        },
        NOON,
    )
    .expect("default on");
    assert!(
        db.settings().expect("settings").reverse_default,
        "the snapshot reads the default back"
    );
    assert_eq!(reverse_queue(&db), vec![card]);

    let refused = db.apply(
        &Command::SetSetting {
            key: "reverse_default".to_string(),
            value: "yes".to_string(),
        },
        NOON,
    );
    assert!(
        matches!(refused, Err(StoreError::Setting { ref key, .. }) if key == "reverse_default"),
        "a value that is neither on nor off is refused: {refused:?}"
    );
}

/// A deck inside another deck.
fn subdeck(db: &mut Db, parent: Id, name: &str) -> Id {
    match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: name.to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("creating a subdeck: {other:?}"),
    }
}

/// The cards whose reverse prompt the new queue would offer, in id order.
fn reverse_queue(db: &Db) -> Vec<Id> {
    let mut cards: Vec<Id> = db
        .new_prompts(0, 0, 1, None)
        .expect("new")
        .into_iter()
        .filter(|prompt| prompt.key == "rev")
        .map(|prompt| prompt.card)
        .collect();
    cards.sort_unstable();
    cards
}

#[test]
fn the_two_directions_keep_separate_schedules() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Directions");
    let card = card(&mut db, deck, "front\n---\nback");

    grade(&mut db, card, "fwd", Grade::Remembered, NOON);
    grade(&mut db, card, "fwd", Grade::Remembered, NOON + TEN_MINUTES);
    grade(&mut db, card, "rev", Grade::Remembered, NOON);
    grade(&mut db, card, "rev", Grade::Forgot, NOON + TEN_MINUTES);

    let forward = db
        .prompt(card, "fwd")
        .expect("fwd prompt")
        .expect("fwd row");
    let reverse = db
        .prompt(card, "rev")
        .expect("rev prompt")
        .expect("rev row");
    assert_eq!(forward.phase, Phase::Review);
    assert_eq!(forward.interval_ms, DAY, "the forward direction graduated");
    assert_eq!(reverse.phase, Phase::Relearn);
    assert_eq!(
        reverse.interval_ms, 0,
        "and the reverse one did not inherit that interval"
    );
    assert_eq!(
        forward.due_at,
        Some(NOON + TEN_MINUTES + DAY),
        "each direction keeps its own due date"
    );
    assert_eq!(reverse.due_at, Some(NOON + 2 * TEN_MINUTES));
}

#[test]
fn history_names_the_direction_of_each_record() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Directions");
    let card = card(&mut db, deck, "front\n---\nback");

    grade(&mut db, card, "fwd", Grade::Remembered, NOON);
    grade(&mut db, card, "rev", Grade::Remembered, NOON + HOUR);

    let history = db.history(card).expect("history");
    assert_eq!(history.len(), 2, "one record per direction");
    assert_eq!(history[0].key, "fwd");
    assert_eq!(history[1].key, "rev");
    assert_eq!(
        db.review_count(card).expect("count"),
        2,
        "the card's count covers both directions"
    );
}

/// Grade one prompt, asserting the store accepted it.
fn grade(db: &mut Db, card: Id, key: &str, grade: Grade, at: i64) {
    match db.apply(
        &Command::Grade {
            card,
            key: key.to_string(),
            grade,
            duration_ms: None,
        },
        at,
    ) {
        Ok(Outcome::Graded { .. }) => {}
        other => panic!("grading {key}: {other:?}"),
    }
}

const TEN_MINUTES: i64 = 10 * 60 * 1000;
const DAY: i64 = 24 * HOUR;

#[test]
fn setting_reverse_on_a_card_or_deck_that_does_not_exist_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);

    let card = db.apply(
        &Command::SetCardReverse {
            card: 404,
            reverse: Some(true),
        },
        0,
    );
    assert!(
        matches!(
            card,
            Err(StoreError::Missing {
                what: "card",
                id: 404
            })
        ),
        "a card that is not there is not silently left inheriting: {card:?}"
    );

    let deck = db.apply(
        &Command::SetDeckReverse {
            deck: 404,
            reverse: Some(false),
        },
        0,
    );
    assert!(
        matches!(
            deck,
            Err(StoreError::Missing {
                what: "deck",
                id: 404
            })
        ),
        "{deck:?}"
    );
}

#[test]
fn a_card_counts_once_against_the_cap_however_many_directions_it_holds() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    db.apply(
        &Command::SetSetting {
            key: "new_per_day".to_string(),
            value: "1".to_string(),
        },
        0,
    )
    .expect("cap to one");
    let deck = deck(&mut db, "Languages");
    db.apply(
        &Command::SetDeckReverse {
            deck,
            reverse: Some(true),
        },
        0,
    )
    .expect("reverse on");
    let both = card(&mut db, deck, "front\n---\nback");

    for seed in 1..12 {
        let queued = db.new_prompts(NOON, 0, seed, None).expect("the queue");
        let mut keys: Vec<&str> = queued.iter().map(|prompt| prompt.key.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["fwd", "rev"],
            "one card's allowance buys both of its directions"
        );
    }

    // The day's ledger agrees: a card that graduated in both directions fills one slot.
    grade(&mut db, both, "fwd", Grade::Remembered, NOON);
    grade(&mut db, both, "rev", Grade::Remembered, NOON);
    assert_eq!(
        db.learned_on(crate::day::study_day(NOON, 0), 0)
            .expect("learned"),
        1,
        "two directions of one card are one card learned"
    );
}

#[test]
fn a_malformed_weight_list_is_refused_without_replacing_the_old_one() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    let good = crate::fsrs::weight_text(&crate::fsrs::DEFAULT_WEIGHTS);
    db.apply(
        &Command::SetSetting {
            key: "fsrs_weights".to_string(),
            value: good.clone(),
        },
        NOON,
    )
    .expect("a list an optimiser could have produced");

    let refused = db.apply(
        &Command::SetSetting {
            key: "fsrs_weights".to_string(),
            value: "0.40255,one,3.173".to_string(),
        },
        NOON,
    );
    match refused {
        Err(StoreError::Setting { key, value }) => {
            assert_eq!(key, "fsrs_weights");
            assert_eq!(value, "0.40255,one,3.173");
        }
        other => panic!("a malformed list should be refused, got {other:?}"),
    }

    let kept: String = db
        .connection_for_test()
        .query_row(
            "SELECT value FROM settings WHERE key = 'fsrs_weights'",
            [],
            |row| row.get(0),
        )
        .expect("the stored list");
    assert_eq!(kept, good, "the old list is still what the library holds");
    assert_eq!(
        crate::fsrs::weights(&kept)
            .expect("and it still parses")
            .len(),
        crate::fsrs::WEIGHT_COUNT
    );
}

#[test]
fn fsrs_state_survives_reopening_the_library() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let card = card(&mut db, deck, "gato\n---\ncat");
    db.apply(
        &Command::SetSetting {
            key: "scheduler".to_string(),
            value: "fsrs".to_string(),
        },
        NOON,
    )
    .expect("choosing FSRS");

    grade(&mut db, card, "fwd", Grade::Remembered, NOON);
    let written = memory_of(&db, card, "fwd");

    drop(db);
    let mut db = open_temp(&dir);
    assert_eq!(
        memory_of(&db, card, "fwd"),
        written,
        "the memory is a column, not something the log could be asked for again"
    );

    // The second grade starts where the first one left off, so the interval grows — and it is
    // graded exactly when it fell due, because a card answered on time has still waited a whole
    // interval.
    let first = interval_of(&db, card, "fwd");
    grade(&mut db, card, "fwd", Grade::Remembered, NOON + first);
    assert!(
        interval_of(&db, card, "fwd") > first,
        "a remembered card should wait longer than it did, got {first} then something shorter"
    );
}

#[test]
fn switching_schedulers_leaves_every_record_alone() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let card = card(&mut db, deck, "gato\n---\ncat");
    grade(&mut db, card, "fwd", Grade::Remembered, NOON);
    let before = db.history(card).expect("history");

    db.apply(
        &Command::SetSetting {
            key: "scheduler".to_string(),
            value: "fsrs".to_string(),
        },
        NOON,
    )
    .expect("choosing FSRS");
    assert_eq!(
        db.history(card).expect("history"),
        before,
        "choosing a scheduler is not a review"
    );

    grade(
        &mut db,
        card,
        "fwd",
        Grade::Remembered,
        NOON + 10 * TEN_MINUTES,
    );
    let after = db.history(card).expect("history");
    assert_eq!(
        after.iter().take(before.len()).count(),
        before.len(),
        "the first record stands"
    );
    assert_eq!(
        db.history(card).expect("history")[0],
        before[0],
        "and is unchanged"
    );
    assert_eq!(after.len(), before.len() + 1, "and exactly one was added");
}

/// The memory the prompt holds, or `None` when no scheduler wrote one.
fn memory_of(db: &Db, card: Id, key: &str) -> Option<(f64, f64)> {
    db.connection_for_test()
        .query_row(
            "SELECT stability, difficulty FROM prompt WHERE card_id = ?1 AND key = ?2",
            rusqlite::params![card, key],
            |row| {
                let stability: Option<f64> = row.get(0)?;
                let difficulty: Option<f64> = row.get(1)?;
                Ok(stability.zip(difficulty))
            },
        )
        .expect("the prompt")
}

fn interval_of(db: &Db, card: Id, key: &str) -> i64 {
    db.connection_for_test()
        .query_row(
            "SELECT interval_ms FROM prompt WHERE card_id = ?1 AND key = ?2",
            rusqlite::params![card, key],
            |row| row.get(0),
        )
        .expect("the prompt")
}

#[test]
fn a_retention_of_one_hundred_and_fifty_percent_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    match db.apply(&Command::SetRetention { percent: 150 }, NOON) {
        Err(StoreError::Setting { key, value }) => {
            assert_eq!(key, "retention");
            assert_eq!(value, "150");
        }
        other => panic!("a target of 150% should be refused, got {other:?}"),
    }
    assert_eq!(
        db.settings().expect("settings").retention,
        crate::store::Settings::DEFAULT_RETENTION,
        "and the refusal must leave the old target alone"
    );
}

#[test]
fn an_unknown_scheduler_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    match db.apply(
        &Command::SetSetting {
            key: "scheduler".to_string(),
            value: "annealing".to_string(),
        },
        NOON,
    ) {
        Err(StoreError::Setting { key, value }) => {
            assert_eq!(key, "scheduler");
            assert_eq!(value, "annealing");
        }
        other => panic!("an unknown scheduler should be refused, got {other:?}"),
    }
    assert!(
        db.settings().is_ok(),
        "the library must still open, so nothing was written"
    );
}

#[test]
fn the_scheduler_is_chosen_by_name_and_the_weights_in_one_go() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.apply(
        &Command::SetScheduler {
            kind: crate::store::SchedulerKind::Fsrs,
        },
        NOON,
    )
    .expect("choosing FSRS");
    db.apply(&Command::SetRetention { percent: 85 }, NOON)
        .expect("a target of 85%");
    let tuned: Vec<f64> = crate::fsrs::DEFAULT_WEIGHTS
        .iter()
        .map(|weight| weight * 2.0)
        .collect();
    db.apply(
        &Command::SetWeights {
            weights: tuned.clone(),
        },
        NOON,
    )
    .expect("an optimiser's list");

    let settings = db.settings().expect("settings");
    assert_eq!(settings.scheduler, crate::store::SchedulerKind::Fsrs);
    assert_eq!(settings.retention, 85);
    assert_eq!(settings.weights, tuned, "the list round-trips as text");
}

#[test]
fn the_export_holds_every_record_with_its_grade() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let card = card(&mut db, deck, "gato\n---\ncat");
    grade(&mut db, card, "fwd", Grade::Forgot, NOON);
    grade(&mut db, card, "fwd", Grade::Remembered, NOON + TEN_MINUTES);
    grade(&mut db, card, "fwd", Grade::Remembered, NOON + DAY);

    let exported = db.export_reviews().expect("export");
    let lines: Vec<&str> = exported.lines().collect();
    let records = usize::try_from(db.record_total().expect("the log")).expect("a count");
    assert_eq!(
        lines[0], "card_id,review_time,grade,phase,interval_ms,due_at,duration_ms",
        "the header names every column the optimiser reads"
    );
    assert_eq!(
        lines.len(),
        records + 1,
        "one row per record and one header, nothing lost and nothing invented"
    );

    // Oldest first, so a file read in order is each card's history in the order it happened.
    assert!(
        lines[1].starts_with(&format!("{card},{NOON},forgot,")),
        "the first record: {}",
        lines[1]
    );
    assert!(
        lines[2].starts_with(&format!("{card},{},remembered,", NOON + TEN_MINUTES)),
        "the second: {}",
        lines[2]
    );
}

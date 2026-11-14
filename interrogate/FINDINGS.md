# Findings — code review of typr at `c77c60b`

Seventeen verified breaks, one dropped claim, five unclaimed items. Every entry was reproduced by
running it before the fix, and each one now has a permanent test in the suite, named in the last
column. The throwaway reproducers were deleted once that coverage landed; they are still in git at
`643f652` (`git show 643f652:interrogate/core-audit.rs`).

No requirement changed to accommodate any of them. Every break contradicted text the live spec already
carried, so the fixes restored the spec rather than amending it, which is why no OpenSpec change was
opened for this work.

Severity means: **high** = a live spec requirement broken, or data lost/duplicated; **medium** =
reachable wrong answer or unhelpful failure; **low** = a diagnostic gap.

## High

| # | Finding | Where | Covered now by |
| --- | --- | --- | --- |
| 1 | A session advances by position, not by the card it graded: `acknowledge` acts on `entries[self.at]`, while `forward`, `backward` and `reshuffle` never consult `awaiting`. A grade lands on the wrong card. | `crates/typr-core/src/session.rs` | `a_grade_lands_on_the_card_it_was_given_for` |
| 2 | A new card graded `forgot` and abandoned is in neither queue. `new_prompts` excludes any prompt holding a `review_log` row, and a learn-phase forgot writes one with `due_at` NULL. | `crates/typr-core/src/store/db.rs` | `a_new_card_forgotten_while_learning_is_still_new` |
| 3 | The app never shows a card's first side: a session is built and never revealed; only the screenshot fixture calls `session.reveal()`. | `crates/typr/src/main.rs:552` | `a_card_landing_at_an_empty_cursor_shows_its_first_side` |
| 4 | A reshuffle hands over a different card with its sides already revealed, because `reshuffle` reorders entries and leaves `revealed` set. | `crates/typr-core/src/session.rs` | `a_reshuffle_never_hands_over_a_revealed_card` |
| 5 | Sessions cannot be scoped to a deck: `Store::due_prompts`/`new_prompts` take no deck and `start_session` filters on `key == "fwd"` alone. | `crates/typr-core/src/store/db.rs`, `crates/typr/src/main.rs` | `a_queue_can_be_scoped_to_one_deck`, `a_new_card_queue_can_be_scoped_to_one_deck`, `a_deck_scope_covers_the_decks_under_it` |
| 6 | Prompts falling due mid-session never join it: `Session::arrive` has no caller outside `session/tests.rs`. | `crates/typr-core/src/session.rs:222` | `a_prompt_that_falls_due_during_a_session_joins_it` |
| 7 | In-progress editor text is never stored, so a restart loses it: `Store::set_draft`/`draft` and `Editor::draft_key` exist and nothing calls them. | `crates/typr/src`, `crates/typr-core/src/store` | `text_typed_and_never_saved_is_there_after_a_restart` |
| 8 | A refused save reads as saved: `persist` falls through to `editor.save()` on the error branch, printing `persist -> true, dirty false, failure Some("the library is closed")`. | `crates/typr/src/main.rs` | `a_save_the_library_refuses_leaves_the_card_dirty` |
| 9 | Tags typed before Save are dropped and the unsaved marker never clears: `persist` sends no tags and `Editor::save()` never updates `saved_tags`. | `crates/typr/src/main.rs`, `crates/typr/src/editor.rs` | `the_tags_typed_before_a_save_go_on_the_card` |
| 10 | Saving a new card twice creates two cards: every `persist` sends `CreateCard` while `editor.card` is still `None`. | `crates/typr/src/main.rs` | `a_second_save_of_a_new_card_does_not_make_a_second_card` |
| 11 | A new card is lost while the app reports it saved: with every root deck archived, the fallback `create_deck(None, "Library")` collides with the archived one and the text stays only in `pending_new_card`. | `crates/typr/src/main.rs` | `a_card_that_still_needs_a_deck_is_not_reported_saved`, `a_deck_send_that_fails_lets_the_waiting_card_go` |

## Medium

| # | Finding | Where | Covered now by |
| --- | --- | --- | --- |
| 12 | The summary's due count is frozen at session start: `due_now` is assigned only in the `Outcome::Due` arm, so the summary says one due while the store says none. | `crates/typr/src/main.rs` | `the_summary_counts_what_is_due_now_not_what_was_due_at_the_start` |
| 13 | A card under an archived deck sits in no view, and a deck archive cannot be undone: the archived view filters on `card.archived`, which a deck archive never sets, and the only sender passes `true`. | `crates/typr/src/main.rs:316` | `a_card_under_an_archived_deck_is_reachable_and_the_deck_can_come_back` |
| 14 | A panic in a later message of a batch is answered against the first request's id, because `in_flight` is the batch's first id: `answered=[1, 1, 0]`. | `crates/typr-core/src/store/task.rs` | `a_panic_on_a_later_message_is_answered_against_that_message` |
| 15 | The failure line for an unreadable library omits the path the spec asks for: "the database refused the statement: file is not a database". | `crates/typr/src/main.rs`, `crates/typr-core/src/store` | `a_library_that_cannot_be_opened_names_its_file`, `a_file_that_is_not_a_database_is_reported_and_left_alone` |
| 16 | Search does not cover tags: `search_text` is `folded(markdown)` only, so `search 'grammar' -> []` while `'subjunctive' -> [1]`. | `crates/typr-core/src/store/db.rs` | `a_search_matches_a_card_by_its_tag` |

## Low

| # | Finding | Where | Covered now by |
| --- | --- | --- | --- |
| 17 | Grades are never timed, so the slow-command record cannot fire for the command the user waits on: `grade answered with id 1; timed commands []`. | `crates/typr/src/main.rs` | `a_grade_is_timed_like_any_other_command` |

## Accounting

- 18 claims received from four adversaries; 17 kept, 1 dropped: adversary 2's "a `relearn` remembered
  is recorded as `review`" — the spec's own scenarios fit the code.
- Three adversaries could not run a shell in their sandbox, so their suspicions were proved by hand;
  the fourth's `probe_the_leave_dialog_closes_a_card_it_could_not_save` passed vacuously (it used a
  missing store where the finding needs a closed one) and was corrected before being kept.
- Unclaimed, not findings: dependency log filtering, the row-builder index panic, a grade in flight
  during a worker panic, whether the preview reflects side boundaries.
- The `cargo test --workspace --locked` suite is green at `c77c60b`: none of the above is covered.

## Running any of these

See `README.md` in this directory.

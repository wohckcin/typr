# Code-review repros

These are the failing repros behind the 17 findings reported for `c77c60b`. They live here, out of
the test targets, so `cargo test --workspace` stays green: every file is an integration test for
`typr-core` and fails on purpose.

Nothing branches on these files. They are evidence, not tests to keep passing.

## Running one

A repro that mutates the library should not touch this checkout, so run it from a throwaway
worktree:

```sh
git worktree add -q --detach /tmp/typr-repro HEAD
cp interrogate/core-adv_repro.rs /tmp/typr-repro/crates/typr-core/tests/
cd /tmp/typr-repro
CARGO_TARGET_DIR=/tmp/typr-repro-target cargo test -p typr-core --test core-adv_repro -- --nocapture
```

Each failing assertion prints the finding it stands for, for example:

```
---- forward_leaves_the_graded_card_behind stdout ----
forward-while-awaiting moved=true remaining=[3, 1]
```

All five files are `typr-core` integration tests: each imports nothing but `typr_core`, so the same
`cp … crates/typr-core/tests/` command works for every one of them.

## What is here

| file | findings |
| --- | --- |
| `core-adv_repro.rs` | session advances by position; reshuffle reveals early; no deck scoping; mid-session arrival missing; wrong id answered; summary count |
| `core-verify_probe.rs` | deck-scoped queues, arrival, summary due count, `acknowledge` against a moved cursor |
| `core-adversarial.rs` | learn-forgot stranding, card identity through grade and ack, deck filters |
| `core-repro.rs` | learn-forgot in a later session, search by tag |
| `core-audit.rs` | drafts never stored, refused save reads as saved, tags before save, double create, lost new card, archived deck, unreadable file message, search by tag, grade timing, panic id |

`adv2-crates_typr_src_tests.rs` and `adv4-crates_typr_src_tests.rs` from the run are emphatically
not here: they were modified copies of `crates/typr/src/tests.rs` carrying probe tests, so dropping
them in would delete the real suite.

## Findings that no file in here covers

Proved with a probe rather than a test, so there is nothing to run: the app never reveals a card's
first side (`crates/typr/src/main.rs:552`), and a session's due count is frozen at session start.

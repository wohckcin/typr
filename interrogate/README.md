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

All five `core-*` files are `typr-core` integration tests: each imports nothing but `typr_core`, so
the same `cp … crates/typr-core/tests/` command works for every one of them. `app-probes.rs` is the
exception — it is a set of functions that must be appended to `crates/typr/src/tests.rs`, because it
uses that module's fixtures.

## What is here

| file | crate | findings |
| --- | --- | --- |
| `core-adv_repro.rs` | `typr-core` | session advances by position; reshuffle reveals early; no deck scoping; mid-session arrival missing; wrong id answered; summary count |
| `core-verify_probe.rs` | `typr-core` | deck-scoped queues, arrival, summary due count, `acknowledge` against a moved cursor |
| `core-adversarial.rs` | `typr-core` | learn-forgot stranding, card identity through grade and ack, deck filters |
| `core-repro.rs` | `typr-core` | learn-forgot in a later session, search by tag |
| `core-audit.rs` | `typr-core` | search by tag; a panicked command's own id answered; learn-forgot stays new; an unreadable library reported with its path |
| `app-probes.rs` | `typr` | drafts never stored; refused save reads as saved; tags typed before save; double create; lost new card when every root deck is archived; card under an archived deck; summary due count; no side shown when a session opens |

The app-level probes are appended to the real test module, never copied over it:

```sh
cat interrogate/app-probes.rs >> /tmp/typr-repro/crates/typr/src/tests.rs   # once only
CARGO_TARGET_DIR=/tmp/typr-repro-target cargo test -p typr --bin typr -- probe_ -- --nocapture
```

Appending twice compiles every probe twice, so start from a fresh worktree if you have already run
it. The two `…src_tests.rs` copies the adversaries left behind are otherwise not kept: they were
whole edited copies of `crates/typr/src/tests.rs`, so dropping one in would delete the real suite.

## Findings that no file in here covers

Grades are never timed, so no slow-command record can fire for the command the user waits on: that
was proved with a one-off probe and there is nothing to rerun. The same is true of the items the
fourth adversary could not verify (dependency log filtering, the row-builder index panic, a grade in
flight during a worker panic, and whether the preview reflects side boundaries): they are unclaimed,
not findings.

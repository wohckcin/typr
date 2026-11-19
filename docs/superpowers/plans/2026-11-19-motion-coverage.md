# Motion Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give each screen that changes state one movement that explains the change, using the existing motion tokens; respect reduced motion at every point.

**Architecture:** Three screens each gain one entrance: the keymap panel and palette enter from their trigger, a newly arrived history record enters at the top, and the editor's saved/unsaved word rises in place. A test then requires every motion token to have a user, and `DESIGN.md` records what plays and what reduced motion does. No new motion system.

**Tech Stack:** Rust, Native Shell motion tokens in `crates/typr/src/ui/motion.rs`, the screen harness in `crates/typr/tests/`.

**Spec:** `docs/superpowers/specs/2026-11-19-migrate-openspec-to-superpowers-design.md`

## Decisions

1. **Keymap panel and palette: enter from the trigger.** The panel arrives with `SLIDE` (8 pt toward the trigger) and `ENTER_SCALE` over `SLOW`, which is what those two tokens describe and what the guide calls a popup's entrance. The row's "the popup's own placement; no app motion" is replaced. Filtering stays instant: a wave per keystroke is noise, and the row keeps that reason.
2. **History: an arriving record enters.** A record added by a review takes `ENTER` over `SLOWER` at the top of the list; the existing list does not move. "Rows are read in one frame" is true of the first read and false of the row that appears while the reader is looking. The filter change keeps its reason.
3. **Card editor: the saved/unsaved word rises.** `RISE` over `FAST` as the word changes — exactly `RISE`'s doc comment ("how far a label rises when it changes"). No glyph swap, so the existing reason's objection does not apply; the row is rewritten to say what plays and what the glyph swap's objection covered. Typing stays instant.
4. **`PRESS_SCALE_ICON` is used or deleted.** No icon button exists today; the smallest honest answer is deletion in this change, and the token returns with the first icon button that needs it. An icon button that exists today may adopt it instead — task 4.1 names both.
5. **A test keeps the vocabulary honest.** `tests/tokens.rs` gains a case that every `pub const` in `ui/motion.rs` is named outside that file, so the next unused token fails a gate instead of sitting in a foundations module.

## Reduced motion

Every added movement is removed under reduced motion, and the state change stays: the panel appears placed, the record appears at the top, the word changes in place. The inventory rows state that per row, as the existing `plays` rows do.

## Global Constraints

- No new motion system, springs, gestures or shaders.
- Per-keystroke updates (typing, palette filter, history filter) stay instant.
- The four screens that already play one movement are not revisited.

## Tasks

### 1. The keymap panel and the palette

- [ ] 1.1 Enter the panel from its trigger: translate by `motion::SLIDE`, scale from `motion::ENTER_SCALE`, over
      `motion::SLOW`, at the placement the popup already computes.
      Verification: `tests/keymap.rs` case `the_panel_enters_from_its_trigger` reads the panel's offset and scale
      on the frame after it opens.
- [ ] 1.2 Reduced motion: the panel appears placed and unscaled.
      Verification: the same test with reduced motion on reads zero offset and scale 1.
- [ ] 1.3 The palette uses the same entrance; filtering stays instant.
      Verification: typing four characters in the palette schedules no transition (the test asserts one entrance
      per opening, not one per keystroke).

### 2. History

- [ ] 2.1 A record that arrives after a review enters with `motion::ENTER` over `motion::SLOWER` at the top of the
      list; the rest of the list does not move.
      Verification: `tests/history.rs` case `an_arriving_record_enters`.
- [ ] 2.2 Reduced motion: the record appears in place.
      Verification: same test, reduced motion on.
- [ ] 2.3 Keep "filter change: none" and its reason.

### 3. The card editor's saved/unsaved word

- [ ] 3.1 The word rises by `motion::RISE` over `motion::FAST` when it changes; no glyph swap.
      Verification: `tests/editor.rs` case `the_saved_word_rises_when_it_changes`.
- [ ] 3.2 Reduced motion: the word changes in place.
      Verification: same test, reduced motion on.
- [ ] 3.3 Typing keeps its "no motion" row.

### 4. The vocabulary

- [ ] 4.1 `PRESS_SCALE_ICON`: delete it from `crates/typr/src/ui/motion.rs`, or apply it to an icon button that
      exists. Record which in design.md Decision 4.
      Verification: `grep -rnw PRESS_SCALE_ICON crates/typr/src` names either a control or nothing.
- [ ] 4.2 `tests/tokens.rs`: a case that every `pub const` in `ui/motion.rs` is named outside it.
      Verification: the case fails when a token is planted unused, passes on the tree after 1–3.

### 5. The inventory

- [ ] 5.1 Rewrite the three screens' rows in `DESIGN.md`: the tool, the token, and what reduced motion does.
- [ ] 5.2 Confirm the rewritten rows in the running app — the panel opening, a review landing, the word changing —
      and tick each as `plays`.
      Verification: each row's `Kind` is `plays` and names what was seen.

### 6. Gate

- [ ] 6.1 Run `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings`,
      `bash scripts/check-tokens.sh` once, at the end.
      Verification: all green; the token check finds no new literal in the three screens.

## Acceptance

### Requirement: A screen that changes state shows the change

A screen SHALL play at least one movement for the state change a reader is most likely to miss, or its
`DESIGN.md` row SHALL state why that change needs none. A row that plays SHALL name the tool, the token and what
reduced motion does.

#### Scenario: A panel opens

- **WHEN** the keymap panel or the palette opens
- **THEN** it enters from its trigger over the token its row names
- **AND** the row states the movement and its reduced-motion form

#### Scenario: A record arrives

- **WHEN** a review is recorded and the history is showing
- **THEN** the arriving record enters at the top of the list
- **AND** the rows already shown do not move

#### Scenario: A status word changes

- **WHEN** the editor's saved/unsaved word changes
- **THEN** the word rises, without a glyph swap

#### Scenario: A change that is not animated

- **WHEN** a screen's own row says no motion
- **THEN** the row gives a reason, and per-keystroke updates are one of them

### Requirement: Reduced motion removes the movement, not the change

Under reduced motion, a movement SHALL be skipped and the state change SHALL still be drawn, at the same layout
position and size the movement would have ended at.

#### Scenario: Every entrance

- **WHEN** reduced motion is on and a panel, a record or a word would enter
- **THEN** it appears placed and unscaled
- **AND** the frame after the change shows the new state

### Requirement: A motion token has a user

A token defined in `crates/typr/src/ui/motion.rs` SHALL be named by at least one screen or control, and SHALL be
deleted when nothing uses it.

#### Scenario: An unused token

- **WHEN** the tokens in `ui/motion.rs` are compared with the code that names them
- **THEN** every one has a user
- **AND** a planted unused token fails the check

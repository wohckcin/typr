# Tasks — motion-coverage

## 1. The keymap panel and the palette

- [ ] 1.1 Enter the panel from its trigger: translate by `motion::SLIDE`, scale from `motion::ENTER_SCALE`, over
      `motion::SLOW`, at the placement the popup already computes.
      Verification: `tests/keymap.rs` case `the_panel_enters_from_its_trigger` reads the panel's offset and scale
      on the frame after it opens.
- [ ] 1.2 Reduced motion: the panel appears placed and unscaled.
      Verification: the same test with reduced motion on reads zero offset and scale 1.
- [ ] 1.3 The palette uses the same entrance; filtering stays instant.
      Verification: typing four characters in the palette schedules no transition (the test asserts one entrance
      per opening, not one per keystroke).

## 2. History

- [ ] 2.1 A record that arrives after a review enters with `motion::ENTER` over `motion::SLOWER` at the top of the
      list; the rest of the list does not move.
      Verification: `tests/history.rs` case `an_arriving_record_enters`.
- [ ] 2.2 Reduced motion: the record appears in place.
      Verification: same test, reduced motion on.
- [ ] 2.3 Keep "filter change: none" and its reason.

## 3. The card editor's saved/unsaved word

- [ ] 3.1 The word rises by `motion::RISE` over `motion::FAST` when it changes; no glyph swap.
      Verification: `tests/editor.rs` case `the_saved_word_rises_when_it_changes`.
- [ ] 3.2 Reduced motion: the word changes in place.
      Verification: same test, reduced motion on.
- [ ] 3.3 Typing keeps its "no motion" row.

## 4. The vocabulary

- [ ] 4.1 `PRESS_SCALE_ICON`: delete it from `crates/typr/src/ui/motion.rs`, or apply it to an icon button that
      exists. Record which in design.md Decision 4.
      Verification: `grep -rnw PRESS_SCALE_ICON crates/typr/src` names either a control or nothing.
- [ ] 4.2 `tests/tokens.rs`: a case that every `pub const` in `ui/motion.rs` is named outside it.
      Verification: the case fails when a token is planted unused, passes on the tree after 1–3.

## 5. The inventory

- [ ] 5.1 Rewrite the three screens' rows in `DESIGN.md`: the tool, the token, and what reduced motion does.
- [ ] 5.2 Confirm the rewritten rows in the running app — the panel opening, a review landing, the word changing —
      and tick each as `plays`.
      Verification: each row's `Kind` is `plays` and names what was seen.

## 6. Gate

- [ ] 6.1 Run `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings`,
      `bash scripts/check-tokens.sh` once, at the end.
      Verification: all green; the token check finds no new literal in the three screens.

# Tasks — keystroke-from-nsh-core

## 1. Proof first

- [x] 1.1 Test `every_binding_is_a_keystroke_nsh_parses` in `crates/typr/tests/keymap.rs`: for every row of
      `typr_core::keymap::bindings()`, `keystroke_of_chord` returns `Some(s)` and
      `nsh_core::keymap::Keystroke::parse_with(&s, true)` succeeds.
      Verification: passes on today's code (it pins the behaviour before the rewrite).

## 2. The conversion

- [x] 2.1 Add `keystroke(chord) -> Option<Keystroke>` in `crates/typr/src/screens/keymap.rs`; make
      `keystroke_of_chord` return its `to_string()`.
- [x] 2.2 Delete `char_keystroke` and the per-key string arms; turn `shift_base` into a `const` slice lookup with
      no `Box::leak`.
      Verification: `grep -n 'Box::leak' crates/typr/src` finds nothing.
- [x] 2.3 Delete `chord_from_keystroke` and its assertions in `tests/keymap.rs:14-27`.
      Verification: `grep -rn chord_from_keystroke crates` finds nothing.
- [x] 2.4 `cargo test -p typr`; the test from 1.1 still passes, and the study and editor key tests pass.

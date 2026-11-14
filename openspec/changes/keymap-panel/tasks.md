# Tasks — keymap, panel, palette

## 1. The table

- [ ] 1.1 Add `crates/typr-core/src/keymap.rs` with `Surface`, `Command`, `Chord`, `Binding`, `bindings()` and `chord_of`, seeded with the bindings that exist today (library `s`/`n`/`h`, editor `Esc`/save, session Space/J/F/Left/Right/R/Esc). Verification: `every_chord_that_exists_today_is_in_the_table` compares the table against the old arms in a test fixture.
- [ ] 1.2 Add `pub fn conflicts(..)` and a test asserting it is empty. Verification: `no_chord_is_bound_twice_on_one_surface`, and a mutation test that a duplicated chord is reported with both commands.
- [ ] 1.3 Implement `Chord::from_event(&KeyEvent)` in `crates/typr/src/keymap/mod.rs`, folding letter case. Verification: `an_uppercase_letter_dispatches_the_lowercase_binding`.

## 2. Dispatch

- [ ] 2.1 Replace `State::on_key`, `study::key` and the editor's `Escape` arm with `keymap::dispatch(surface, chord)` plus one executor per surface. Verification: the existing keyboard tests (`a_whole_queue_is_answered_with_no_pointer`, `escaping_the_editor_returns_to_the_library_and_asks_first_when_dirty`) still pass unchanged.
- [ ] 2.2 Delete the hand-written `study::BINDINGS` row and draw it from the table. Verification: `the_session_draws_the_bindings_the_table_holds` compares rendered text against `bindings()`.

## 3. Panel and keycaps

- [ ] 3.1 Add `crates/typr/src/keymap/panel.rs`: a `Dialog` grouped by surface with a filter field, opened and closed by `ShowShortcuts`. Verification: `the_panel_opens_and_closes_on_its_key` and `the_panel_filter_narrows_the_list`.
- [ ] 3.2 Draw a keycap beside every control whose command has a binding (library header buttons, session answer buttons). Verification: `the_reveal_action_shows_its_key`.
- [ ] 3.3 Hide keycaps below the narrower width. Verification: `a_narrow_window_keeps_the_labels_without_the_keycaps`.

## 4. Palette

- [ ] 4.1 Add `crates/typr/src/keymap/palette.rs`: `Cmd+K` opens it, typing filters over labels, Up/Down move the highlight, Enter runs the command through the surface executor. Verification: `typing_narrows_and_enter_runs` and `a_session_command_is_not_offered_on_the_library`.
- [ ] 4.2 Register scenes `shortcut-panel` and `command-palette`, both snapped non-empty. Verification: two `--snap` runs.

## 5. Verification

- [ ] 5.1 Run the gate: fmt, clippy with `-D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene. Verification: all four report success.

## Workflow follow-up

- `deck-views` adds its bindings to this table; land this change first. Archive with
  `openspec archive keymap-panel --yes` once checked.

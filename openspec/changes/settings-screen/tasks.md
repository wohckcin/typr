# Tasks — settings screen

Depends on `review-reverse` and `fsrs-scheduling` for the keys it exposes.

## 1. Typed settings

- [x] 1.1 `SettingKey { NewPerDay, LearningStepMinutes, MaxIntervalDays, Scheduler, Retention, Weights, ReverseDefault, Theme, Scale, PreviewFontSize }` with `parse`, `as_str`, `default_value`, and `Settings` grown to match; one validation function per group in `crates/typr-core/src/store/settings.rs`.
  - **Verification**: `cargo test -p typr-core --lib settings` covers each key's default, each range's two edges, a refused value keeping the old one, and a stored garbage value being reported.
- [x] 1.2 `Command::SetSetting { key: SettingKey, value: SettingValue }`, all callers and tests migrated.
  - **Verification**: `every_setting_round_trips` writes each key and reads `Settings` back; `an_unknown_key_cannot_be_stored` no longer compiles a string key, asserted by the type alone and recorded in the task note.
  - Note: `an_unknown_key_cannot_be_stored` is not a test any more because a `&str` does not type-check where a `SettingKey` is expected; `SettingKey::parse("new_cards")` returning `None` is covered by `a_text_value_no_build_writes_names_the_key_and_the_text`.

## 2. Screen

- [x] 2.1 `Screen::Settings` and `crates/typr/src/settings.rs` with three sections; opened from the library and from a session, returning to the card that was up.
  - **Verification**: `settings_open_from_a_session_and_return_to_the_card` drives the key path in `rune::Runtime` and asserts the session's card is unchanged.
- [x] 2.2 Appearance reaches the theme and the scale on the next frame, with a reset control.
  - **Verification**: `a_scale_change_is_drawn_on_the_next_frame` compares element geometry before and after; `a_the_theme_change_repaints` reads the kit's dark flag.
- [ ] 2.3 Library section shows the database path, schema version and migration note, size, attachments and log paths, plus import and export.
  - **Verification**: `the_settings_screen_reports_the_files_schema` seeds a rewound library and reads the drawn text; the reveal control is covered by a scene and by hand, and its headless gap is stated in the task note.

## 3. Close out

- [ ] 3.1 A scene `settings` renders the screen with a library open; README documents each key and its default.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 3.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the changes before it.
- Next roadmap row: `visual-parity`.

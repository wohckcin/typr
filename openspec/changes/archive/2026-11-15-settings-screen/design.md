# Design — settings screen

## Context

`Settings` in `crates/typr-core/src/store/mod.rs` is a `Copy` struct with one field, `new_per_day`, read
into every `Snapshot` and consumed by `new_prompts`. Values live in `settings(key TEXT PRIMARY KEY, value
TEXT NOT NULL)`. `Command::SetSetting { key: String, value: String }` is the only writer and it already
refuses a non-numeric cap with `StoreError::Setting { key, value }`.

Screens are a four-way selector (`Screen { Library, Editor, Session, Summary }`) read in one place in
`crates/typr/src/main.rs`, and the theme comes from `rune::tok::Theme` with `Kit::neutral(dark)`.

## Goals / Non-Goals

**Goals**

- Every setting the clone changes already added becomes reachable and validated.
- A refused value cannot corrupt a run.
- Paths and versions visible where a user looks first.

**Non-Goals**

- Per-deck settings beyond the reverse switch that already exists; a deck's own copy of every setting is
  a different feature.
- Profiles, presets or importing settings from a file.
- Keybinding remapping, which needs the binding table generalised first.

## Decisions

**`SetSetting` becomes typed: `SettingKey` enum plus a typed value.** A free-text key lets the UI store a
value nothing reads, and a free-text value means every reader re-validates. `SettingKey` with
`parse`/`as_str` and a `value` accessor means a misspelt key fails to compile, and the store validates
ranges in one place.

**`Settings` stays a plain struct read once per snapshot.** It is already `Copy` and published inside
`Arc<Snapshot>`; growing it to a dozen fields costs nothing per frame and keeps every reader unchanged in
shape. The alternative, a `HashMap<String, String>` in the snapshot, would put parsing back in the views.

**The screen is a fifth `Screen` variant, not a dialog.** It carries its own key handling and returns to
whichever screen it was opened from, which a modal layering over a live review would not.

**Appearance applies through the theme, not through per-widget overrides.** `Settings::theme` picks
`Kit::neutral(dark)` and the interface scale feeds Rune's scale factor; a widget reading its colours from
the kit then follows for free.

**Paths come from the desktop crate's `paths` module and the store's `schema_facts`.** The backend
already refuses to know about `AppDirs` (project-structure invariant), so the screen reads its paths from
the caller side and its schema numbers from the store, which is the split that already exists.

## Risks / Trade-offs

- A typed settings enum is a migration of every existing `SetSetting` caller and test; the compile error
  is the checklist, and the stored keys are unchanged, so no data moves.
- Interface scale can be set to a value that makes the app unusable; the range is clamped and a "Reset"
  control exists, which is cheaper than a confirm-on-every-change.
- Revealing a path in the file manager is platform-specific and cannot be tested headlessly; it is
  covered by a scene and by hand, and stated as unproven in the change's tasks.
- A settings change that needs a reload (the scheduler) takes effect on the next grade rather than
  instantly, which is stated on screen instead of hidden.

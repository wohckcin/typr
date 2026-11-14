# Tasks — Mochi import and export

Depends on `card-templates` (fields), `diagram-cards` (masks) and `cloze-deletions` (variant keys).

## 1. Library export

- [x] 1.1 `transfer::export_library(db_path, target) -> Result<(), StoreError>` using `VACUUM INTO`, refusing an existing target and reporting a write failure with its path.
  - **Verification**: `the_export_reopens_with_every_record` exports a fixture library, opens the export with `Db::open`, and compares decks, cards, prompts and record counts; `exporting_leaves_the_library_untouched` compares modification time and `schema_facts()`.

## 2. Mochi import

- [x] 2.1 `transfer::mochi::manifest(archive) -> Result<Manifest, StoreError>` reading the format version and declared counts first, refusing an unknown version.
  - **Verification**: `a_newer_export_version_is_refused` and `a_truncated_archive_is_refused_before_any_write` (the second asserts the library's card count before and after).
- [x] 2.2 Decks and cards: create decks with nesting, then each card through `Db::apply`, mapping text, templates (as a card template), fields and tags.
  - **Verification**: `three_nested_decks_and_forty_cards_import_whole` and `an_imported_card_renders_through_its_template`.
- [x] 2.3 Variants and history: map each Mochi variant to a prompt key, write its records with their times, and let `refresh_prompt` derive the due date.
  - **Verification**: `imported_history_keeps_its_grades_and_times` and `a_card_reviewed_both_ways_imports_both_prompts`; `an_imported_due_date_matches_its_last_record`.
- [x] 2.4 Images and masks: copy each referenced image into the attachments folder, rewrite its name, and import masks as fractional rectangles.
  - **Verification**: `an_imported_image_is_present_after_its_source_is_deleted` and `imported_masks_keep_their_group_and_position`.
- [x] 2.5 Count validation: compare what the manifest declared with what was written and fail the whole import on a mismatch.
  - **Verification**: `a_manifest_count_mismatch_imports_nothing`.

## 3. Text formats

- [x] 3.1 CSV export and import with the documented header, refusing a file whose header lacks a required column.
  - **Verification**: `a_csv_round_trip_keeps_sides_tags_and_fields` and `a_csv_without_the_header_is_refused`.
- [x] 3.2 Markdown export per deck and import into a chosen deck, preserving the separator and hidden-text syntax.
  - **Verification**: `markdown_round_trip_reproduces_prompts` asserts the same prompt keys and `sides` after import.

## 4. UI and close out

- [x] 4.1 The library offers Import and Export through the native file dialog, runs the work on the store thread, and reports progress and failure on the failure line.
  - **Verification**: `importing_a_mochi_export_from_the_library_adds_its_cards` drives the command path and settles the answer; an unreadable file leaves the count unchanged and the reason on screen.
- [x] 4.2 README documents the four formats, the CSV header and what a `.typr` file is; the new dependency is recorded with its reason.
  - **Verification**: the README section exists and `cargo tree -p typr-core --edges normal` shows the archive crate and no toolkit crate.
- [x] 4.3 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the changes before it.
- Next roadmap row: `settings-screen`.

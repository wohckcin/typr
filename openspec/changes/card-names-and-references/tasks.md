# Tasks — card names and references

Depends on `card-templates` for name templates built from fields, and on `custom-views` for the list
that shows names.

## 1. Names

- [x] 1.1 New `crates/typr-core/src/name.rs`: `pub fn for_card(first_side: &str, template: Option<&str>, fields: &Fields<'_>) -> String` truncating at 120 characters on a word boundary, plus `pub fn unique(name: &str, taken: &[&str]) -> String` adding " (2)".
  - **Verification**: `cargo test -p typr-core --lib name` covers first-side naming, template naming, a template that renders empty (falling back to the first side), truncation, and the suffix.
- [x] 1.2 Migration V9: `card.name TEXT NOT NULL DEFAULT ''` with `CREATE INDEX card_name ON card(name COLLATE NOCASE)`, backfilled for existing cards from their first side, and `card.name_template TEXT`.
  - **Verification**: `every_card_that_existed_gains_a_name` rewinds a real file, reopens, and finds no empty names; `a_stored_name_follows_an_edit` saves a changed first side and sees the name change.

## 2. References

- [x] 2.1 New `crates/typr-core/src/reference.rs`: `pub fn links(text: &str) -> Vec<&str>` collecting `[[…]]` outside fences and inline code, and `pub fn rewrite(text: &str, resolved: &HashMap<String, Id>) -> String` producing markdown links with the `typr-card:` scheme.
  - **Verification**: `links_ignores_code` and `rewrite_leaves_unresolved_text_alone`.
- [x] 2.2 Migration V10: `reference(from_card INTEGER NOT NULL, to_card INTEGER NOT NULL, PRIMARY KEY (from_card, to_card))` with both foreign keys cascading; `Db::rebuild_references(card)` deletes and re-inserts that card's rows inside the save transaction; `Db::backlinks(card) -> Vec<CardSummary>`.
  - **Verification**: `a_saved_card_records_its_references` and `a_backlink_disappears_when_the_target_is_renamed_away`.
- [x] 2.3 Resolution order: the referencing card's deck, then its ancestors, then the library, then the oldest card.
  - **Verification**: `resolution_prefers_the_nearest_deck` builds two same-named decks and asserts the chosen target.

## 3. Tags as paths

- [x] 3.1 Tag parsing keeps a whole path in one tag; `Snapshot` exposes a tag tree; a filter by a tag includes its descendants.
  - **Verification**: `filtering_a_parent_includes_its_children` and `a_tag_path_is_one_tag` assert one tag row with two segments.

## 4. Screens

- [ ] 4.1 Rows and the editor header show the card's name; the row still shows the preview as its second line.
  - **Verification**: `the_list_shows_the_name_even_when_the_preview_differs`.
- [ ] 4.2 The editor renders references as controls, opens the target on use, and lists backlinks in a panel.
  - **Verification**: `a_reference_opens_the_card_it_names`, `an_unresolved_reference_says_so`, `the_backlink_panel_lists_every_referrer` — each driven through `rune::Runtime`.
- [ ] 4.3 A reference picker completes names while typing between `[[` and `]]`.
  - **Verification**: `typing_two_brackets_offers_names_and_a_choice_fills_them`.

## 5. Close out

- [ ] 5.1 A scene `editor-backlinks` shows a card with three referrers; README documents names, references and tag paths.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 5.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the changes before it.
- Next roadmap row: `fsrs-scheduling`.

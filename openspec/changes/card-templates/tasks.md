# Tasks — card templates

Depends on `cloze-deletions` for the second sigil's meaning and on nothing else.

## 1. Render

- [x] 1.1 New `crates/typr-core/src/template.rs`: `pub struct Fields<'a>(…)` over `&[(String, String)]`, `pub struct Context<'a> { pub name: Option<&'a str>, pub deck: Option<&'a str> }`, and `pub fn render(template: &str, fields: &Fields<'_>, context: &Context<'_>) -> String` replacing `<<Field>>`, skipping empty sections, keeping an unclosed section as text, and leaving `{{…}}` alone so hidden text survives.
  - **Verification**: `cargo test -p typr-core --lib template` covers each filled and unfilled placeholder, a section with and without a value, an unclosed section, a dynamic field, a field named like a dynamic one, and a template containing `{{1::x}}` unchanged.
- [x] 1.2 Recompute on write: a store helper `fn rerender(&self, card: Id) -> Result<(), StoreError>` sets `markdown`, `sides` (through `sides::split`) and `search_text` (folded markdown plus folded field values), called from every mutation that can change them.
  - **Verification**: `the_stored_text_is_what_the_template_renders` asserts the invariant after a field edit; `a_deck_template_change_rewrites_its_cards` and `a_deck_move_rerenders_against_the_new_template`.

## 2. Store

- [x] 2.1 Migration V7: `card.template TEXT`, `deck.template TEXT`, and `field(card_id INTEGER NOT NULL, name TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY (card_id, name))` with a cascading foreign key.
  - **Verification**: `a_library_from_before_templates_renders_as_before` rewinds a real file to `user_version = 6`, reopens, and finds every card's text and queue unchanged; `opening_reports_what_the_schema_did` counts the step.
- [ ] 2.2 Commands `SetFields { card, fields: Vec<(String, String)> }`, `SetCardTemplate { card, template: Option<String> }`, `SetDeckTemplate { deck, template: Option<String> }`; `CardSummary` and `Snapshot` carry fields and which template is in force.
  - **Verification**: `a_card_reports_the_template_it_inherits` and `a_fifty_card_deck_renders_in_one_pass` (the second asserts fifty rendered cards and that each holds the prompt its text deserves).

## 3. Editor

- [ ] 3.1 A card whose deck (or own setting) has a template shows a field table beside the rendered preview, and the markdown pane becomes read-only with a note saying the template supplies it.
  - **Verification**: `the_field_table_renders_the_card_live` types into a field and finds the value in the preview; `a_card_under_a_template_does_not_offer_a_raw_markdown_pane`.
- [ ] 3.2 A deck's template is edited from the sidebar, with a preview of one of its cards.
  - **Verification**: `a_deck_template_is_edited_and_its_cards_follow` saves a template and finds the cards' rendered text changed after a refresh.
- [ ] 3.3 Clearing a deck template or a card's own template leaves every card readable.
  - **Verification**: `clearing_a_template_leaves_the_cards_readable`.

## 4. Close out

- [ ] 4.1 README gains a table of the three syntactic forms (separator, hidden text, template) and a scene `editor-template` renders a card with a field table.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 4.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the three changes before it.
- Next roadmap row: `custom-views`.

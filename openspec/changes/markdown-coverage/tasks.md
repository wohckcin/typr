# Tasks — markdown coverage

## 1. Parser

- [ ] 1.1 Widen the parser options in `crates/rune-kit/src/markdown/parser.rs` to add superscript, subscript and definition lists. Verification: `cargo test -p rune-kit markdown` still passes and a new upstream test asserts a superscript event is emitted for `20^th^`.
- [ ] 1.2 Commit the upstream work on Rune's `dev`, bump `rune`/`rune-kit` and the four `[patch.crates-io]` entries to the new revision together. Verification: `cargo build --workspace --locked` from an empty target, and `cargo tree -p typr-core --edges normal` still names no toolkit crate.

## 2. Rendering

- [ ] 2.1 Add `highlight: Background` to `MarkdownStyle` and render highlight runs with it. Verification: upstream test `a_highlighted_run_is_drawn_with_the_highlight_style`.
- [ ] 2.2 Render superscript and subscript as raised and lowered runs. Verification: `superscript_is_raised` and `subscript_is_lowered` in the upstream renderer tests.
- [ ] 2.3 Render definition lists as a term and its indented definitions. Verification: `a_term_with_two_definitions_keeps_both`.
- [ ] 2.4 Render `InlineMath` and `DisplayMath` as a set-apart monospace run with the source intact. Verification: `inline_math_keeps_its_source` and `display_math_gets_its_own_line`.
- [ ] 2.5 Render ruby as a base with its reading above. Verification: `a_ruby_gloss_draws_the_reading_over_the_base`.

## 3. Preprocessing

- [ ] 3.1 Add `crates/typr-core/src/markdown_ext.rs` with `preprocess(text) -> Cow<str>` rewriting `==` and ruby, tracking fences and code spans. Verification: `equals_inside_code_are_left_alone`, `a_fence_is_left_alone`, `ruby_needs_braces_not_just_parentheses`.
- [ ] 3.2 Make the transform idempotent and assert it in a test. Verification: `preprocessing_twice_changes_nothing`.
- [ ] 3.3 Call `preprocess` from the one place cards are rendered, so the editor preview, the review and the tile all see the same text. Verification: `the_preview_and_the_review_render_the_same_highlight`.

## 4. Degradation and inventory

- [ ] 4.1 Add `crates/typr-core/src/markdown_ext/supported.rs`: `pub const SUPPORTED: &[(&str, Support)]` listing each construct as rendered, degraded or unsupported, and a test that every entry has a case in the renderer tests. Verification: `every_supported_construct_has_a_case`.
- [ ] 4.2 Assert unknown constructs keep their literal text. Verification: `an_unknown_construct_is_shown_as_written`.

## 5. Verification

- [ ] 5.1 Add a scene `markdown-kitchen-sink` whose card uses every supported construct, and confirm it snaps non-empty and differs from `editor`. Verification: one `--snap` run plus a size comparison.
- [ ] 5.2 Run the gate: fmt, clippy with `-D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene. Verification: all four report success.

## Workflow follow-up

- Archive with `openspec archive markdown-coverage --yes` once checked. If upstream refuses highlight or ruby,
  record the fallback in `design.md` before archiving rather than leaving the task unchecked.

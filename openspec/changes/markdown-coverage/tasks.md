# Tasks — markdown coverage

## 1. Parser

- [ ] 1.1 Superscript and subscript parser options. Blocked: `rune::Span` has no baseline shift, so a parsed superscript could only be drawn smaller, not raised. typr keeps the Unicode fallback (`20ᵗʰ`, `H₂0`) in `typr-core` until Rune gains a shift. Verification: not met.
- [x] 1.2 Vendor rune-kit at `vendor/rune-kit` (root `[patch."https://github.com/anomalyco/rune"]`), with its delta recorded in `vendor/rune-kit/PATCHES.md`. Verification: `cargo build --workspace --locked` and `cargo tree -p typr-core --edges normal` names no toolkit crate.

## 2. Rendering

- [x] 2.1 Highlight runs drawn on `MarkdownStyle::mark_bg` (vendored). Verification: `a_highlighted_run_is_drawn_with_the_highlight_style` in `vendor/rune-kit/src/markdown/tests.rs`.
- [ ] 2.2 Superscript and subscript as raised and lowered runs. Blocked on the same missing `rune::Span` baseline shift as 1.1. Verification: not met.
- [x] 2.3 Definition lists: the term bold, each definition its own paragraph. Verification: `a_term_with_two_definitions_keeps_both` in `vendor/rune-kit/src/markdown/tests.rs`.
- [x] 2.4 Inline and display math as a set-apart monospace code run, source kept (display math flattened to one run, since a code run cannot hold a line break). Verification: `inline_math_keeps_its_source` and `display_math_is_one_run_on_its_own_line`.
- [ ] 2.5 Ruby as a base with its reading above. Blocked: needs stacked inline layout that `rune::Span` cannot express. typr keeps `base (reading)`. Verification: not met.

## 3. Preprocessing

- [x] 3.1 Add `crates/typr-core/src/markdown_ext.rs` with `preprocess(text) -> Cow<str>` rewriting ruby, superscript and subscript, tracking fences and code spans. Verification: `equals_inside_code_are_left_alone`, `a_fence_is_left_alone`, `ruby_needs_braces_not_just_parentheses`.
- [x] 3.2 Make the transform idempotent and assert it in a test. Verification: `preprocessing_twice_changes_nothing`.
- [x] 3.3 Call `preprocess` from the one place cards are rendered, so the editor preview, the review and the tile all see the same text. Verification: `the_preview_and_the_review_render_the_same_highlight`.

## 4. Degradation and inventory

- [x] 4.1 Add `crates/typr-core/src/markdown_ext/supported.rs`: `pub const SUPPORTED: &[(&str, Support)]` listing each construct as rendered, degraded or unsupported, and a test that every entry has a case in the renderer tests. Verification: `every_supported_construct_has_a_case`.
- [x] 4.2 Assert unknown constructs keep their literal text. Verification: `an_unknown_construct_is_shown_as_written`.

## 5. Verification

- [x] 5.1 Add a scene `markdown-kitchen-sink` whose card uses every supported construct, and confirm it snaps non-empty and differs from `editor`. Verification: one `--snap` run plus a size comparison.
- [x] 5.2 Run the gate: fmt, clippy with `-D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene. Verification: all four report success.

## Workflow follow-up

- Archive with `openspec archive markdown-coverage --yes` once checked. If upstream refuses highlight or ruby,
  record the fallback in `design.md` before archiving rather than leaving the task unchecked.

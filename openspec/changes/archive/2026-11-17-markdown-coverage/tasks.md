# Tasks — markdown coverage

## 1. What the renderer draws today

- [x] 1.1 Render one card holding each construct through `text_view` headlessly and record what each one shows.
      Verification: the table is in `design.md`'s Context — highlight, superscript and ruby literal; subscript,
      both maths and definition lists as the renderer draws them.

## 2. The three constructs as declared extensions

- [x] 2.1 Declare `MarkdownExtension::pattern` for highlight (`==([^=\n]+)==`), superscript (`\^([^\^\n]+)\^`) and
      ruby (`\{([^}\n]*)\}\(([^)\n]*)\)`), each with its own node name.
      Verification: a unit test parses a card holding all three and finds one custom node per construct.
- [x] 2.2 Draw each node in `render_custom`: a background for highlight, smaller raised text for superscript, the
      reading above the base for ruby.
      Verification: headless render of a card per construct shows the text and none of the markers.
- [x] 2.3 Wire the extensions and the renderer into the one place cards are drawn
      (`crates/typr/src/ui/prose.rs:17-22`), so the editor preview, the review and the tile all see them.
      Verification: `the_preview_and_the_review_render_the_same_highlight`.

## 3. One list of constructs

- [x] 3.1 Restate `crates/typr-core/src/markdown_ext/supported.rs`'s rows against the observed renderer:
      subscript and both maths as rendered, highlight and ruby as this change draws them, superscript as rendered
      or degraded by 3.2, definition lists and raw HTML as unsupported.
      Verification: every row names the test that covers it.
- [x] 3.2 Record whether superscript is drawn raised or only smaller; if inline text cannot be raised, its row says
      `Degraded`.
      Verification: the record matches what 2.2 renders.
- [x] 3.3 A test fails when the list and the renderer tests disagree.
      Verification: `every_supported_construct_has_a_case`, and `tests/markdown.rs` draws one source per row
      (tables, task lists, links, quotes, raw HTML, footnotes and the spec's two "leave it alone" cases included),
      so a row cannot point at a test that never draws it.

## 4. The pre-pass that nothing calls

- [x] 4.1 Delete `crates/typr-core/src/markdown_ext/preprocess.rs` and its `preprocess/` tests; the parser now
      reads the same syntax, and nothing has called `rewrite` since the renderer changed.
      Verification: `grep -rn 'preprocess\|markdown_ext::rewrite' crates --include=*.rs` finds nothing.
- [x] 4.2 Keep the rest of `markdown_ext` (`expand`, `embeds`, `plain`, `INFO`, `MISSING`); it serves card embeds
      and tile previews and is unrelated to this change.

## 5. Gate

- [x] 5.1 Run `cargo test --workspace`, `nsh test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check` and `bash scripts/check-tokens.sh` once, at the end.
      Verification: all five report success.

## Workflow follow-up

- Archive with `openspec-archive-change markdown-coverage` once every box is checked; the change is applied
  before it is archived, not after.

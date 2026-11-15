# Local patches to rune-kit

Vendored from `anomalyco/rune` at revision `764fbe9e75ebc91ab94c830181920826fb417be0`
(`crates/rune-kit`). Upstream grants this project no write access, so the delta lives here.

## What changed

- `src/markdown/mark.rs` (new): `==text==` highlights. Before parsing, each opening and closing `==` becomes
  a Unicode noncharacter (U+FDD0 / U+FDD1). Code spans and fenced blocks are left as written; a highlight
  does not span a line; an unpaired marker is written back as `==`.
- `src/markdown/markdown.rs`: `Style::Mark`; `Markdown::new` runs the highlight rewrite on its source.
- `src/markdown/parser.rs`:
  - sentinels in text events open and close a `Style::Mark` run; an unpaired opener is written back as `==`
    at its position (`insert_at` shifts spans, links and the source map);
  - sentinels that reach an indented code block, a math body or a link or image destination are written back
    as `==` (`mark::restore`). This is a stopgap: pairing on text events in the parser would exclude these
    by construction;
  - definition lists: the term is bold, each definition is its own paragraph;
  - `InlineMath` is a code run keeping its `$…$`; `DisplayMath` is one code run keeping its `$$…$$`, with
    newlines flattened to spaces (a code run cannot hold a line break and would wrap per line);
  - options `ENABLE_DEFINITION_LIST` and `ENABLE_MATH` are enabled.
- `src/markdown/render.rs`: a `Style::Mark` run becomes a span with the `mark_bg` background.
- `src/markdown/style.rs`: `mark_bg` (default: accent at 25% alpha) and `mark_pad` fields, hashed with the
  rest of the style; `MarkdownStyle::mark_background(color)` builder.
- `assets/fonts/`: copies of `InterVariable.ttf` and `IBMPlexMono-Regular.ttf` from the Rune checkout, so the
  crate's own tests (`src/markdown/tests.rs`) compile here.
- `Cargo.toml`: a self-contained manifest (its own `[workspace]`, explicit edition and license, rune crates
  from the pinned git revision, and a standalone `[patch.crates-io]` block that standalone builds need).
  `examples/` and `tests/` are removed.

## Behaviour changes to know about

- `Markdown::new(src)` stores the **rewritten** source. Byte ranges given to `Markdown::highlights` (`find`)
  refer to that rewritten text, so a source with `==` pairs has shifted offsets compared with the caller's
  string.
- A card with no `==` is unchanged; `mark::rewrite` borrows and does not allocate.

## Testing

`cargo test --manifest-path vendor/rune-kit/Cargo.toml --features markdown --lib` runs the crate's own
tests (135 at the time of writing), including the mark, definition-list and math cases.

## Re-applying on a Rune bump

1. Copy the new `crates/rune-kit` into `vendor/rune-kit`, keeping `assets/` and the manifest.
2. Re-apply the files listed above. `mark.rs` is self-contained.
3. Keep the manifest's git revision in step with the root `Cargo.toml` and `crates/typr/Cargo.toml`.

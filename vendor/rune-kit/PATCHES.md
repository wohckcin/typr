# Local patches to rune-kit

Vendored from `anomalyco/rune` at revision `764fbe9e75ebc91ab94c830181920826fb417be0`
(`crates/rune-kit`). Upstream grants this project no write access, so the delta lives here.

## What changed

- `src/markdown/mark.rs` (new): `==text==` highlights. The source is rewritten before parsing so each
  opening and closing `==` becomes a Unicode noncharacter (U+FDD0 / U+FDD1). Code spans and fenced
  blocks are left as written; a highlight does not span a line; an unpaired marker stays as `==`.
- `src/markdown/markdown.rs`: `Style::Mark`; `Markdown::new` runs the rewrite on its source.
- `src/markdown/parser.rs`: sentinels in text events open and close a `Style::Mark` run; a sentinel
  that reaches an indented code block is written back as `==`.
- `src/markdown/render.rs`: a `Style::Mark` run becomes a span with the `mark_bg` background.
- `src/markdown/style.rs`: `mark_bg` (default: accent at 25% alpha) and `mark_pad` fields, hashed with
  the rest of the style; `MarkdownStyle::mark_background(color)` builder.
- `Cargo.toml`: a self-contained manifest (`[workspace]` root, explicit edition and license, rune
  crates from the pinned git revision). `examples/` and `tests/` are removed; the in-crate unit tests
  remain.

## Behaviour changes to know about

- `Markdown::new(src)` stores the **rewritten** source. Byte ranges given to `Markdown::highlights`
  (`find`) refer to that rewritten text, so a source that contains `==` pairs has shifted offsets
  compared with the caller's string.
- A card with no `==` is unchanged; `mark::rewrite` borrows and does not allocate.

## Re-applying on a Rune bump

1. Copy the new `crates/rune-kit` into `vendor/rune-kit`.
2. Re-apply the files listed above (`mark.rs` is self-contained).
3. Keep the manifest's git revision in step with the root `Cargo.toml` and `crates/typr/Cargo.toml`.

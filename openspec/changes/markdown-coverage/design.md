# Design — markdown coverage

## Context

typr renders cards through `rune_kit::markdown`: `Markdown::new(source).render(&MarkdownStyle::neutral(dark))`,
with a content-keyed `parse_cached`. The parser is constructed with
`Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS`
(`crates/rune-kit/src/markdown/parser.rs:466`, pulldown-cmark 0.13.6).

pulldown-cmark 0.13.6 already offers `ENABLE_SUPERSCRIPT`, `ENABLE_SUBSCRIPT`, `ENABLE_DEFINITION_LIST`,
`ENABLE_MATH` (emitting `InlineMath`/`DisplayMath`), `ENABLE_GFM`, `ENABLE_HEADING_ATTRIBUTES` and
`ENABLE_FOOTNOTES`. It has no notion of `==highlight==` or of ruby, so those two need handling outside the
parser.

Mochi's editor accepts, per its own shortcut panel: `==highlight==`, `20^th^`, `H~2~0`, `{base}(reading)`,
`$..$` and `$$..$$`, definition lists (`Cmd+D`), plus the constructs this app already renders.

## Goals

- A card written for Mochi reads the same here for every construct except LaTeX typesetting.
- Unsupported syntax is visible, never silently missing.
- One list of supported constructs, used by both the renderer and the tests.

## Non-Goals

- No LaTeX typesetter; math is preserved, not laid out.
- No raw HTML, no footnotes, no GFM tables beyond what tables already give.
- No change to how cards are stored: `card.markdown` is unchanged text.

## Decisions

### Parser options are widened first, because they are free

`ENABLE_SUPERSCRIPT`, `ENABLE_SUBSCRIPT` and `ENABLE_DEFINITION_LIST` are one line in the parser and give
pulldown-cmark events for three of the six constructs. Doing them by hand was rejected: a hand parser for `^`
would have to re-implement escaping, code spans and nesting that pulldown-cmark already gets right.

### `==` and ruby are rewritten before parsing, with a guard

`crates/typr-core/src/markdown_ext.rs` exposes `fn preprocess(text: &str) -> Cow<'_, str>`, which rewrites
`==x==` and `{base}(reading)` into marker forms the parser understands and leaves everything else byte-identical.
It walks the text tracking fenced-block state and inline-code spans, because a rewrite that fires inside code
would corrupt it — a test asserts that `` `a == b` `` survives unchanged.

### Highlight needs a style, so it goes through the renderer's own hook

`MarkdownStyle` gains one field, `highlight: Background`, defaulting to the kit's accent at low alpha, and the
renderer draws the run with that background. Reusing emphasis was rejected: bold and highlighted must be
distinguishable, since a card can carry both.

### Definition lists and math fall back to structure we already draw

pulldown-cmark emits `DefinitionList`/`DefinitionListTitle`/`DefinitionListDefinition` and
`InlineMath`/`DisplayMath`. The renderer maps a definition list to a term paragraph plus indented paragraphs, and
math to a monospace run set apart — the same shape as inline code with a different style, so the text is legible
and the difference from prose is visible. Rendering math as prose was rejected because `\\pi` then reads as an
escape.

### Upstream changes are a planned step, not a surprise

Three renderer additions (highlight, ruby, math) live in `rune-kit`, so the change carries an explicit upstream
task and a pinned-revision bump. `project-structure` requires one revision across the workspace, so the bump
touches the root `[patch.crates-io]` entries and both members together. If upstream refuses the additions, the
fallback is stated in the task: render highlight as bold, ruby as `base (reading)`, and record the divergence —
never drop the text.

### Upstream has no write path, so rune-kit is vendored with a recorded delta

`anomalyco/rune` grants this account read-only access, and `rune-kit`'s style fields are `pub(super)`, so
no outside crate can add a highlight background or a definition layout. The toolkit's markdown renderer
is therefore vendored at `vendor/rune-kit` with its delta listed in `vendor/rune-kit/PATCHES.md`.

What the vendored renderer draws, and what is still a fallback:

| Construct | Drawn as | Status |
|---|---|---|
| `==text==` | background span (`mark_bg`) | Rendered |
| definition list | bold term, definitions as paragraphs | Rendered |
| `$x$` | inline code run, delimiters kept | Degraded (no typesetter) |
| `$$x$$` | one code run, delimiters kept, on its own line | Degraded |
| `20^th^`, `H~2~0` | Unicode raised/lowered where every character has one, else literal | Degraded (typr-core) |
| `{base}(reading)` | `base (reading)` | Degraded (typr-core) |

Superscript, subscript and ruby stay degraded because `rune::Span` has no baseline shift and no stacked
layout. Those need a Rune change. Tasks 1.1, 2.2 and 2.5 stay unchecked for that reason.

## Risks / Trade-offs

- **Preprocessing rewrites user text.** Guarded by the code-span tracker and by tests over fences, nesting and
  escaped markers; the transform is idempotent, which a test asserts by running it twice.
- **A revision bump rebuilds everything.** 233 crates, a few minutes; already routine in this repo.
- **Math looks like code.** Accepted: it is legible and never corrupts prose, and the typesetter is a later
  change.

## Migration Plan

None. Rendering only; stored markdown is untouched.

## Open Questions

- Whether ruby should also be readable as plain text for screen readers instead of an overlay.

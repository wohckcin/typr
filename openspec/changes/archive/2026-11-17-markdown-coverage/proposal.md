# Proposal — markdown coverage

## Why

Cards are written for Mochi, whose shortcut panel documents the syntax its editor accepts. This app draws them
through `nsh_base::text_view` (`crates/typr/src/ui/prose.rs:20`), and that renderer already covers most of it:
markdown-rs 1.0 with `ParseOptions::gfm()` plus math text and flow
(`nsh-base-0.5.1/src/text_view/markdown/mod.rs:247-255`), so CommonMark, tables, task lists, strikethrough,
autolinks and footnotes render today. Math is read by the parser and drawn by the view's own formula path
(`text_view/view.rs:754-796`); what this build draws for `$..$` — typeset, terminal text, or source in a box —
is not yet observed, and task 1.1 records it.

Observed on a headless `text_view` (`screen().texts()`), three constructs are left over and each shows its own
markers:

| Construct | Written as | Shown as |
|---|---|---|
| Highlight | `==the Krebs cycle==` | `==the Krebs cycle==` |
| Superscript | `20^th^ century` | `20^th^ century` |
| Ruby | `{base}(reading)` | `{base}(reading)` |

The same probe shows two constructs the renderer already handles: `H~2~0` is drawn as `H2O` with the tilde
markers consumed, and `$E=mc^2$` and `$$…$$` are drawn as formula elements, not as their TeX source.

A card imported from Mochi that says `==important==` reads as `==important==`: a content regression, not a
cosmetic one. markdown-rs 1.0 has no construct for any of the three, and none for definition lists either
(`markdown-rs 1.0.0/src/configuration.rs:39`, `Constructs`).

The pre-pass written for the previous renderer is dead: `crates/typr-core/src/markdown_ext/preprocess.rs` has no
caller (`grep -rn 'preprocess' crates --include=*.rs` finds only its own module and tests).

## What Changes

- `==highlight==`, `^superscript^` and `{base}(reading)` are declared as `MarkdownExtension`s on the text view's
  parser, and the app draws the nodes they make through `render_custom`.
- The declared inventory moves beside those extensions: one table of every construct, rendered, degraded or
  unsupported, with the renderer test that covers each.
- `crates/typr-core/src/markdown_ext/preprocess.rs` is deleted; nothing calls it.
- Definition lists stay unsupported and are recorded as such, since markdown-rs has no construct for them.

## Capabilities

### New Capabilities

- `markdown-coverage`: which markdown constructs render, how unknown constructs degrade, and where the list of
  supported constructs lives.

### Modified Capabilities

None. `card-editor`'s "A card's text is markdown" stays true; this change widens what that sentence means.

## Impact

- `crates/typr/src/ui/prose.rs`: the text view gains `.extension(..)` and `.render_custom(..)`.
- A new module, `crates/typr/src/ui/prose/extensions.rs`: the three `MarkdownExtension`s, their node names, and
  the `render_custom` renderer.
- `crates/typr-core/src/markdown_ext/`: `preprocess.rs` deleted; `supported.rs` replaced by the inventory next to
  the extensions, or kept with its rows restated against the NSH renderer.
- No behavioural change to scheduling, storage or the review loop.

## Explicitly out of scope

LaTeX typesetting is not ours to build, and math is not this change's: the parser reads `$..$` and `$$..$$`
(`text_view/markdown/latex.rs` rewrites TeX's `\(…\)` and `\[…\]` into dollars), and the probe above shows the
view drawing them as formula elements in this build. Nothing about math changes here.

# Design — markdown coverage

## Context

A card's text is drawn by `crates/typr/src/ui/prose.rs:17-22`: `text_view().markdown(move || Some(source))`.
The view parses with markdown-rs 1.0, `ParseOptions::gfm()` plus math text and flow
(`nsh-base-0.5.1/src/text_view/markdown/mod.rs:247-255`), and reads `\(…\)`/`\[…\]` as dollars
(`text_view/markdown/latex.rs`).

Observed on a headless `text_view` (task 1.1), before any change:

| Construct | Written | Drawn |
|---|---|---|
| Highlight | `==hl==` | `==hl==` |
| Superscript | `20^th^` | `20^th^` |
| Subscript | `H~2~0` | `H20` — read by the renderer already |
| Ruby | `{base}(reading)` | `{base}(reading)` |
| Inline math | `$E=mc^2$` | a formula element, no source text |
| Display math | `$$…$$` | a formula element |
| Definition list | `term` then `: definition` | `term : definition` |

What the 0.5.1 crate offers for syntax it does not read itself:

- `MarkdownExtension` (`text_view/extension.rs:20-46`): inline syntax as a `pattern` (Rust `regex`, matched in
  text outside code and links), or a fenced `language`, an HTML `tag`, a `container`. Each match becomes a
  `CustomNode`.
- `MarkdownPlugins` (`extension.rs:213-246`): Rust plugins given to the view with
  `provide_context(MarkdownPlugins::new().with(plugin))`; `configure` sets `ParseOptions`, `inline` reads a node
  itself, `render` draws it.
- `render_custom: Prop<Option<Renderer<CustomNode>>>` (`text_view/view.rs:78`): the app draws a custom node.

Leftovers from the rune era: `crates/typr-core/src/markdown_ext/preprocess.rs` has no caller
(`grep -rn 'preprocess' crates --include=*.rs` finds only its own module and tests), and
`markdown_ext/supported.rs` lists constructs as the old renderer saw them.

Math: markdown-rs reads `$..$`/`$$..$$` (math text and flow are on). Drawing it is a feature question —
`nsh-base-0.5.1/Cargo.toml:29-36` defaults to `editor`, `highlight`, `plot`, `rich-editor`, `term` and **not**
`math`; `view.rs:531-536`'s `formula()` returns `None` unless the formula renderer is ready, and the arms after it
draw terminal text (`tex_text`) or the TeX in a box.

## Goals

- A card written for Mochi reads the same here for every construct except LaTeX layout.
- Syntax the renderer cannot draw stays visible as written, never silently missing.
- One list of supported constructs, checked against the renderer's tests.

## Non-Goals

- No new markdown parser: the extensions are the parser's own hook.
- No change to how cards are stored: `card.markdown` is unchanged text.
- No LaTeX layout built here.

## Decisions

1. **The three constructs are declared extensions, not a pre-pass.** `MarkdownExtension::pattern` is a regex
   matched by the parser "in text, outside code and links" (`extension.rs:24-28`), which is exactly the guarantee
   `markdown_ext::preprocess` re-implemented by hand (tracking fences and code spans) and which nothing has called
   since the renderer change. Declaring `==([^=]+)==`, `\^([^^]+)\^` and the ruby pattern also leaves the parser's
   escaping and nesting intact. Subscript needs no pattern: the renderer reads `~…~` itself (table above).
2. **The nodes' look is the app's.** Superscript is smaller text lifted three points inside an inline view: the
   renderer has no baseline shift, so the list keeps it `Degraded` rather than claiming a raised run (task 3.2).
    `render_custom` receives the node's text and attributes and returns the
   element: highlight is a background on the run, superscript and subscript are smaller text in a shifted run,
   ruby is a column with the reading above the base. If an inline view cannot be raised, the construct is recorded
   as degraded in the inventory rather than drawn wrong (task 3.2 records which).
3. **Ruby is one pattern with two groups.** `\{([^}]*)\}\(([^)]*)\)` gives the base as group 1 and the reading as
   group 2, so the renderer needs no second parse; text without the brace-and-parenthesis shape matches nothing
   and stays as written.
4. **The inventory stays one table, beside the extensions.** Each row names a construct, its support
   (`Rendered`, `Degraded`, `Unsupported`) and the test that covers it, including the constructs the NSH renderer
   itself draws, so the list answers for the whole surface and not only for what this change adds.
   `markdown_ext/supported.rs`'s rows are restated against this renderer; its `Support` enum is kept.
5. **Definition lists are unsupported.** markdown-rs 1.0 has no construct for them
   (`markdown-rs 1.0.0/src/configuration.rs:39`, `Constructs`), so a term-and-definition pair keeps its literal
   text and the inventory says so.
6. **Math is the renderer's, and this build draws it.** The probe showed `$E=mc^2$` and `$$…$$` drawn as formula
   elements, so `view.rs:531-536`'s `formula()` path is available in this dependency graph even though
   `nsh-base`'s own default feature list (`Cargo.toml:29-36`) omits `math`. Nothing to enable, nothing to build;
   the inventory records math as rendered by the renderer.

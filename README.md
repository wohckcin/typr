# typr

A local-first flashcard app in the shape of Mochi: write markdown cards, review them on a spaced
schedule, keep every byte on your own disk. Built on [Native Shell](https://nsh.dev).

Status: the port from Rune to Native Shell is merged; what the app promises lives in
`openspec/specs/`, and a change in flight in `openspec/changes/`.

## Layout

Two crates in one workspace, so the backend can be tested and reused without a graphics stack:

| Crate | What it is | Normal dependencies |
|---|---|---|
| `crates/typr-core` | scheduling, sessions, sides, templates, the SQLite library | `rusqlite`, `thiserror`, `unicode-normalization`, `serde`, `zip` |
| `crates/typr` | the screens, the `ui` layer and the app's root | `nsh-core`, `nsh-base`, `nsh-platform-macos` or `nsh-platform-linux`, `log`, `typr-core` |

The backend resolves no directories and reads no time zone: `Db::open` takes a path and every
study-day query takes an offset, both supplied by the app in `crates/typr/src/paths.rs`.

Screens are drawn only from `crates/typr/src/ui`, and only with the theme's tokens: `tests/tokens.rs`
fails on a spacing, type, radius or colour literal, or on a `nsh_base` control used directly.

## Prerequisites

- [mise](https://mise.jdx.dev), which installs the pinned tools: Rust `1.99.1` and the `nsh` CLI
  `0.5.1`, both from `mise.toml`.

  ```sh
  mise install     # Rust and nsh, at the versions mise.toml pins
  mise exec -- nsh test desktop.test.tsx   # or activate mise in the shell and drop `mise exec --`
  ```

  `rust-toolchain.toml` carries the same Rust pin for a machine without mise.
- **Linux only:** `libgtk-4-dev` (GTK 4.14 or newer), GStreamer's dev packages and their runtime
  libraries, to build and run; CI installs them and the screens run headless.

## Build and run

```sh
cargo run -p typr        # the app window (crates/typr/src/bin/typr_nsh.rs)
nsh run                  # the same app through the nsh development host
```

## Tests

Behaviour is tested headlessly: each test opens the app on the real engine, presses, types and reads
what is on screen. There are no screenshots and no scene files.

```sh
cargo test --workspace                  # the whole workspace, headless
cargo test -p typr appearance           # one test binary
nsh test desktop.test.tsx               # the app driven through its own screens, headless
nsh test                                # the app crate's Rust tests, then the TSX files
nsh test --target macos desktop.test.tsx # the same steps in an AppKit window
nsh check                               # type checks plus `cargo check --all-targets`
```

`desktop.test.tsx` reads the app rather than writing to it: the real host opens the library in your
data folder, so its tests only press keys and assert what is on screen. A headless run has no data
folder, and the shell states that ("Library closed") while still showing the library's grid. The
field's contents cannot be read back on the macOS host, so the test proves the search field is
reachable and the chord works, not what the field then holds.

`nsh test` runs the app crate's Rust tests (154) and then the TSX files, but not `typr-core`'s: the
workspace's 483 tests are `cargo test --workspace`'s job, and `nsh test --target` runs the TSX files
only, so neither substitutes for it.

Appearance is checked by the styles a screen resolves to, not by pictures: `tests/appearance.rs`
renders every screen in light and dark and asserts that the same text keeps its size and weight and
resolves to the theme's colour for that appearance.

## Writing cards

A card is markdown. Two things give it structure:

- A line that is exactly `---` splits the card into sides. A study session reveals the sides in order
  and grades once, after the last one. A card with no `---` is a note: it never enters a queue.
- `{{hidden text}}` hides text until you reveal it. One unnumbered span is one prompt; numbered
  spans `{{1::this}}` and `{{2::that}}` are separate prompts, each with its own schedule.
- Hidden text wins over the separator: a card that hides text is asked as hidden text, however many
  lines of `---` it holds.
- Spans inside fenced code blocks and inside `backticks` are literal text, not hidden text.

### The three syntactic forms

| Form | What it does |
|---|---|
| A line of `---` | Starts the next side. One separator splits every card in a deck, so the renderer splits the rendered text, not the template. |
| `{{hidden text}}` | Hides text until you reveal it; `{{1::this}}` numbered groups are separate prompts. |
| `<<Field>>` | A template field, filled per card from the field table. `{{#Field}}…{{/Field}}` shows the block only while the field has a value. |

A card whose deck (or its own settings) carries a template is rendered from it: its text is the
template with the fields filled in, stored as it renders, so a review never re-derives it.

## Scheduling

Two schedulers answer a grade. The fixed one is the default and the one every earlier library was
scheduled with: a first success waits ten minutes, a repeated one stretches by two and a half, a miss
falls back to ten minutes. The other is FSRS-5, which keeps a stability and a difficulty per prompt
and schedules from them against a target recall.

Both are set in the `settings` table. Every key, its range and its default:

| Key | Values | Default | What it does |
| --- | --- | --- | --- |
| `new_per_day` | 0 to 1000 | `20` | new cards a new-card session offers |
| `learning_step_minutes` | 1 to 1440 | `10` | how long a missed card waits before it comes back |
| `max_interval_days` | 1 to 3650 | `365` | the longest interval either scheduler will set |
| `scheduler` | `fixed`, `fsrs` | `fixed` | which scheduler answers a grade |
| `retention` | 70 to 97 | `90` | the recall FSRS aims at; higher means sooner |
| `fsrs_weights` | nineteen positive numbers | the FSRS-5 defaults | what the optimiser produced |
| `reverse_default` | `0`, `1` | `0` | whether a new card is reviewed both ways |
| `theme` | `dark`, `light` | `dark` | the window's appearance |
| `scale` | 0.80 to 1.50 | `1.0` | interface scale |
| `preview_font_size` | 10 to 28 | `14` | body text size in the editor's preview |

A value is validated before it is written: a number outside its range, a theme or scheduler the
reader does not know, a scale that is not a positive number, or a weight list that is not nineteen
positive numbers is refused, the screen says which setting and which value, and the old setting stays.
A value edited into the file by hand is read through the same validators, so a library with a bad row
reports the key and text instead of silently replacing it.

The history can be exported for tuning — `Db::export_reviews` returns CSV, one row per record, in the
schema the FSRS optimiser reads.

## Views and decks

A view is a list you keep: a deck's subtree, a search, a tag, a card mode and an order, saved under a
name. Decks nest, and every card belongs to exactly one deck. Tags are paths: `spanish/verbs` is one
tag with two segments, listed under `spanish`, and filtering by `spanish` includes the cards tagged
with any of its children.

`[[Name]]` refers to another card; the store resolves it when the card is saved, preferring the card's
own deck, then the decks above it, then the whole library.

## Where the app keeps its data

The app's data folder comes from Native Shell; `TYPR_DATA_DIR` overrides it. Inside it:

| Path | What it holds |
|---|---|
| `typr.sqlite3` | the library: decks, cards, tags, settings and review history |
| `attachments/` | the pictures a card's markdown references |
| `logs/` | reserved for the app's log; nothing writes it yet |

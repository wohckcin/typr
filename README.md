# typr

A local-first flashcard app in the shape of Mochi: write markdown cards, review them on a spaced
schedule, keep every byte on your own disk. Built on [Rune](https://github.com/anomalyco/rune).

Status: early. What the app promises lives in `openspec/specs/`; the work in flight lives in
`openspec/changes/`.

## Layout

Two crates in one workspace, so the backend can be tested and reused without a graphics stack:

| Crate | What it is | Normal dependencies |
|---|---|---|
| `crates/typr-core` | scheduling, sessions, sides, the SQLite library | `rusqlite`, `thiserror`, `unicode-normalization` |
| `crates/typr` | the Rune window, the views, the scenes | `rune`, `rune-kit`, `time`, `typr-core` |

The backend resolves no directories and reads no time zone: `Db::open` takes a path and every
study-day query takes an offset, both supplied by the app in `crates/typr/src/paths.rs` and
`day_offset.rs`.

## Prerequisites

- Rust `1.99.1` — pinned by `rust-toolchain.toml`, because Rune pins it.
- **Linux only:** development packages `libxkbcommon-dev libwayland-dev` to link, and
  `libxkbcommon-x11-0 mesa-vulkan-drivers` for a windowed run. Without the XKB X11 library every
  windowed start panics with `Library libxkbcommon-x11.so could not be loaded`. Headless
  `--snap` runs need neither.

## Build and run

```sh
cargo run -p typr
```

Headless, no window — the same renderer, one frame captured to a PNG:

```sh
cargo run -p typr -- --snap /tmp/typr.png
cargo run -q -p typr -- --list-scenes
cargo run -p typr -- --scene history --snap /tmp/history.png
```

A headless run never opens the library, so it writes nothing under the app's data folder.

Debug builds always accept the headless arguments. A release build needs Rune's `tooling`
feature to take them:

```sh
cargo build -p typr --release --features rune/tooling
./target/release/typr --list-scenes
./target/release/typr --scene summary --snap /tmp/summary.png
```

`--script` replays input, and `--video` records the frames the script produces. Each event is a
JSON object: `{"key": "remembered"}` taps an element by its key, `{"text": "…"}` types into the
focused field, `{"physical": "Escape"}` presses a key, and a `"t"` field places it in seconds.

## Writing cards

A card is markdown. Two things give it structure:

- A line that is exactly `---` (nothing else on it) splits the card into sides. A study session
  reveals the sides in order and grades once, after the last one. A card with no `---` is a note:
  it never enters a queue.
- `{{hidden text}}` hides text until you reveal it. One unnumbered span is one prompt; numbered
  spans `{{1::this}}` and `{{2::that}}` are separate prompts, each with its own schedule,
  because each one is a different question.
- Hidden text wins over the separator: a card that hides text is asked as hidden text, however
  many lines of `---` it holds.
- Spans inside fenced code blocks and inside `backticks` are literal text, not hidden text.

While you type, the preview masks every answer and names the group it belongs to — `[1 … ]` for a
numbered span, `[ … ]` for the plain form — and the caption counts prompts rather than sides — so a card with two numbered groups reports two prompts, not two
sides.

## Reverse reviews

A two-sided card can be asked from either end. Reverse is off until you turn it on, and it is
decided in three places, nearest first:

- the card's own setting, from the tile's `⋯` menu (`Ask both ways` / `Ask one way`);
- the deck's setting, from the sidebar's `Reverse: …` row, which offers `On`, `Off` and `Inherit`;
- `reverse_default` in the settings table, which is what a deck inherits when its own value is NULL.

A card that inherits follows its parent deck's value, so turning reverse on for `Languages` turns
it on for `Japanese` unless `Japanese` says otherwise. When a card is asked from both ends, its
tile's footer reads `BOTH WAYS`.

Each direction is its own prompt with its own schedule and history: graduating `front → back` does
not move `back → front`, and the session heading names which way the card is being asked. Turning
reverse off neither deletes nor resets the reverse prompt's records — it stops being queued, and
turning it back on resumes where it left off. The new-card limit counts prompts, so a card asked
both ways takes two of the day's slots.

## Keyboard

Every binding lives in one table, `typr_core::keymap::TABLE`, and the app draws its hints from
it: press `?` on any screen to see the list filtered, or `⌘K` (Ctrl+K on Linux and Windows) for
the command palette. Keycaps beside a button disappear when the window is narrower than 520
points, and the action stays.

| Screen | Key | Action |
|---|---|---|
| Library | `s` | Review what is due |
| Library | `n` | Review the new cards |
| Library | `c` | New card |
| Editor | `s` | Save the card |
| Editor | `h` | Show the card's history |
| Editor | `Alt+N` | Add a rectangle to a diagram |
| Editor | `Alt+Left` / `Alt+Right` | Narrow or widen the selected rectangle |
| Editor | `Alt+Up` / `Alt+Down` | Shorten or heighten the selected rectangle |
| Editor | `Esc` | Back to the library, asking first when the card is unsaved |
| Session | `Space` / `Enter` | Reveal the next side |
| Session | `j` | Remembered |
| Session | `f` | Forgot |
| Session | `Right` / `Left` | Next card / back, read-only |
| Session | `r` | Shuffle the cards still to answer |
| Session | `Esc` | Back to the library |
| Summary | `Esc` | Start a session from the list |
| Every screen | `?` | Keyboard shortcuts |
| Every screen | `⌘K` | Command palette |

A session keeps the keyboard: while a session is running the library's keys do nothing, and while
the panel or the palette is open nothing else answers a key.

## Dependencies

`rune` and `rune-kit` are pinned to one Rune `dev` revision in `crates/typr/Cargo.toml`, and
`[patch.crates-io]` at the workspace root repeats the four crates Rune patches (`winit`,
`parley_engine`, `wgpu-hal`, `accesskit_windows`) from that same revision — a dependency's `[patch]`
does not reach this workspace, and a patch section is only honoured at a workspace root, so without
them the build would silently use stock crates. When Rune's root `[patch.crates-io]` gains an entry,
add it here.

`dev` is fast-forward-only, so a revision is a stable target and upgrades are deliberate: bump the
revision, re-read Rune's `[patch.crates-io]`, rebuild.

## Logs

The app writes one log file, `<logs>/typr.log`, where `<logs>` is Rune's per-app logs folder:
`~/Library/Logs/typr` on macOS, `$XDG_STATE_HOME/typr/logs` (by default
`~/.local/state/typr/logs`) on Linux, `%LOCALAPPDATA%\typr\Logs` on Windows, and
`<root>/logs/typr` when `RUNE_DIRS_ROOT` is set. It holds what the screen cannot: the cause chain
behind a failure line, the startup facts (database path, the schema version found and written, how
many migrations ran, the log path), any command whose answer took longer than 250 ms, and how the
library worker ended.

`TYPR_LOG` sets the level (`error`, `warn`, `info`, `debug`, `trace`); unset or unparseable means
`warn`. The file is rotated once on open when it is larger than 1 MiB: the old one becomes
`typr.log.1`, replacing any previous copy. Records are handed to a writer thread through a bounded
channel, so a full channel drops records rather than stalling a frame; the next successful write
reports how many were dropped.

## Tests

```sh
cargo test --workspace
```

The backend's tests run without a display or a GPU driver:

```sh
cargo test -p typr-core
```

The domain layer takes its time from a clock it is given (`typr_core::clock::Clock`), so "due in ten
minutes" is an assertion rather than a sleep.

## Working on it

```sh
openspec list --specs               # what the app promises
openspec list                       # active changes
openspec instructions apply --change typr-workspace --json
```

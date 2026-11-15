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

### The three syntactic forms

| Form | What it does |
|---|---|
| A line of `---` | Starts the next side. One separator splits every card in a deck, so the renderer splits the rendered text, not the template. |
| `{{hidden text}}` | Hides text until you reveal it; `{{1::this}}` numbered groups are separate prompts. |
| `<<Field>>` | A template field, filled per card from the field table. `{{#Field}}…{{/Field}}` shows the block only while the field has a value. |

A card whose deck (or its own settings) carries a template is rendered from it: its text is the
template with the fields filled in, stored as it renders, so a review never re-derives it. `Name`
and `Deck` are computed — the card's first side and its deck's name — and appear in the field table
as read-only rows, so what the card will say is visible before it is saved.
Clearing a template leaves every card saying what it last rendered.

While you type, the preview masks every answer and names the group it belongs to — `[1 … ]` for a
numbered span, `[ … ]` for the plain form — and the caption counts prompts rather than sides — so a card with two numbered groups reports two prompts, not two
sides.

## Names, references and tags

Every card has a name: the first line of its first side, cut at a readable length, or what its own
name template renders (`<<Word>> (<<Kind>>)` for a language deck). Names are unique inside a deck —
the second `Introduction` becomes `Introduction (2)` — while two decks may each hold one; the name is
stored when the card is written, so renaming a deck never renames its cards. Lists show the name over
the preview, and the editor header shows it too.

`[[Name]]` refers to another card. The store resolves it when the card is saved, preferring the
card's own deck, then the decks above it, then the whole library, so the same text can point at
different cards in different decks. A resolved reference is drawn as a link that opens the target;
its text is stored as written, so resolving never rewrites the card. A name that matches no card
stays plain text and the editor lists it as unresolved. Every target lists who refers to it in a
**Referenced by** panel under the preview, and typing `[[` offers the names in the library to finish
the reference.

Tags are paths: `spanish/verbs` is one tag with two segments, listed under `spanish`, and filtering
by `spanish` includes the cards tagged with any of its children.

## Diagram cards

A card can also be a diagram: switch the editor to **Diagram**, choose a picture, and draw
rectangles on it. Each rectangle belongs to a group, and each group is one prompt with its own
schedule, so a single picture can hold several questions. During a review the rectangles of the
group being asked are covered, the rest of the picture stays in sight, and one reveal lifts the
covers.

Draw with the pointer by dragging across the picture, or from the keyboard: `Alt+N` places a
rectangle in the middle and the `Alt` arrow chords size it. **Merge** puts a rectangle in the
previous one's group, **Remove** takes it off the picture, and the caption under the picture says
what it shows. A group with no rectangles left is retired, and the reviews already answered on it
stay on disk.

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

## Scheduling

Two schedulers answer a grade. The fixed one is the default and the one every earlier library was
scheduled with: a first success waits ten minutes, a repeated one stretches by two and a half, a
miss falls back to ten minutes. The other is FSRS-5, which keeps a stability and a difficulty per
prompt and schedules from them against a target recall; a card it has never seen but which already
has an interval keeps that interval by deriving its stability from it, so switching does not reset
what the fixed scheduler learned.

Both are set in the `settings` table, which the settings screen (`,` from the library or a session)
exposes. Every key, its range and its default:

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
| `scale` | 0.80 to 1.50 | `1.0` | interface scale; layout is laid out in window/scale points |
| `preview_font_size` | 10 to 28 | `14` | body text size in the editor's Markdown preview |

A value is validated before it is written: a number outside its range, a theme or scheduler the
reader does not know, a scale that is not a positive number, or a weight list that is not nineteen
positive numbers is refused, the screen says which setting and which value, and the old setting
stays. Nothing is written on a refusal, so a library never holds a value that would stop it
opening. A value edited into the file by hand is read through the same validators, so a library
with a bad row reports the key and text instead of silently replacing it.

The screen's Library group reports the file itself rather than a setting: the database path, the
schema version the file is at, its size, the attachments folder and the log path, each with a
control that shows it in the file manager, plus the import and export dialogs. When an older
library is opened, the schema row says which version it was at before this open brought it forward.

A scheduler change takes effect on the next grade; the other keys apply on the next frame.

FSRS state lives beside the prompt, in two nullable columns: a prompt that has not been through
FSRS has no memory rather than a made-up one, and grading it under the fixed scheduler leaves those
columns untouched, so switching back and forth does not lose what either scheduler learned.

The history can be exported for tuning — `Db::export_reviews` returns CSV, one row per record, in
the schema the FSRS optimiser reads:
`card_id,review_time,review_rating,review_state,review_duration`. The rating is the optimiser's
number (1 for again, 3 for good) rather than this app's word, the state is the one the prompt was in
*before* the grade (0 New, 1 Learning, 2 Review, 3 Relearning), and the duration is empty when the
review was not timed. Every column is a number, so no cell needs quoting. The optimiser itself is not
in the app: that file goes out, a weights list comes back in through `fsrs_weights`, and the next
FSRS grade uses it.

## Views

A view is a list you keep: a deck's subtree, a search, a tag, a card mode and an order, saved under
a name in the sidebar's `Views` section.

Press `New view`, type a name, and the list you are looking at becomes the view. Selecting a view
applies every part of it at once — it clears any deck you had selected and takes over the search
field, the tag filter, the `Active`/`Archived`/`Trash` tabs and the order. `Rename` and `Delete` sit
on the row; deleting the view you are looking at returns to the whole library.

Five orders are offered above the grid: `Last edited` (newest first), `Created`, `First side`
(alphabetical), `Due date` (earliest first, unscheduled cards last) and `Reviews` (most reviewed
first). The arrow beside them flips the direction. Changing the order while a view is selected
rewrites that view's order and nothing else — a search you typed is not saved with it. `Shuffle`
deals the list in a random order for this run only; leaving the view or picking an order restores it.

A card's star in the tile's footer pins it, and `☆ Pinned only` under the views lists pinned cards
alone. Pinning is a property of the card, not of the view, so it survives the trash and every view
you open; the pinned-only filter is part of a view and is remembered in it.

A filter typed while a view is selected is not an edit of the view: only the order and the
pinned-only flag are written back.

A view remembers its deck by id. If that deck is deleted, the view says so on its row and shows the
whole library rather than nothing.

## Cramming

`x` in the library studies exactly the cards the filter selects — the deck, the search, the tag and
the pinned-only flag that are on screen — ignoring due dates, the new-card cap and the scheduler.
With nothing selected it refuses and says so rather than opening an empty run.

A cram answer writes nothing: no prompt changes phase, takes an interval or gains a due date, and
`review_log` gains no row. The only thing a run changes is its own queue, so forgetting a card puts
it back at the end of the same run and a card leaves only when it is remembered. The summary says
how many answers were given and that nothing was scheduled, and it can save the run's filter as a
view: the name is yours to type, and nothing is saved unless you press the button.

## Import and export

The library screen has **Import** and **Export** beside the other filter controls. Both open the
native file dialog; the file's extension decides the format.

- **Import** reads a Mochi `.mochi` archive, a `.csv` file, or a `.md`/`.markdown` file. CSV and
  Markdown write into the deck selected in the sidebar; a `.mochi` archive brings its own decks.
- **Export** writes the whole library as one `.typr` file, or the selected deck as `.csv` or
  `.md`. The line at the bottom of the screen says what was written, or why the export failed.

A `.typr` file is the library itself — decks, cards, prompts, fields, masks and every review
record — copied by SQLite, so it reopens exactly as it was. It is not a backup of your pictures:
images live beside the library in its own folder and are not inside the file, so a `.typr` opened
on another machine shows the cards but not their pictures.

CSV and Markdown are for reading and editing text outside the app. Both carry cards' text, sides,
tags and template fields, and nothing else: diagram cards are left out because no column carries a
picture or a mask, and templates, review history, reverse settings and pins stay behind. CSV has a
header — `deck, sides, tags, fields, markdown` — where tags and fields are JSON cells so a comma or
a newline inside one does not split the row; `deck` records where the row came from, and importing
writes into the deck you have selected. Markdown separates cards with a blank line and starts each
file with the deck's name as a heading, so a card that itself contains a blank line cannot round
trip through Markdown (use CSV or `.typr` for those).

Importing adds. It does not merge, update or skip cards that are already there, so importing the
same file twice gives you the cards twice. A duplicate deck name inside one Mochi archive is
refused rather than silently merged, and a refused import writes nothing.

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
| Library | `x` | Cram the cards the filter selects |
| Library | `,` | Open settings |
| Library | `Down` / `j` | Select the next card |
| Library | `Up` / `k` | Select the previous card |
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
| Session | `,` | Open settings |
| Session | `Esc` | Back to the library |
| Summary | `Esc` | Start a session from the list |
| Settings | `Esc` | Close settings and return to the screen that opened them |
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

`typr-core` also depends on `zip` (with the pure-Rust `zlib-rs` deflate backend) and `serde_json`:
a Mochi archive is a zip of JSON, and the import has to read it on the store's worker thread, which
cannot use the UI crate's readers.

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

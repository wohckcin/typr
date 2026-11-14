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

## Dependencies

`rune` and `rune-kit` are pinned to one Rune `dev` revision in `crates/typr/Cargo.toml`, and
`[patch.crates-io]` at the workspace root repeats the four crates Rune patches (`winit`,
`parley_engine`, `wgpu-hal`, `accesskit_windows`) from that same revision — a dependency's `[patch]`
does not reach this workspace, and a patch section is only honoured at a workspace root, so without
them the build would silently use stock crates. When Rune's root `[patch.crates-io]` gains an entry,
add it here.

`dev` is fast-forward-only, so a revision is a stable target and upgrades are deliberate: bump the
revision, re-read Rune's `[patch.crates-io]`, rebuild.

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

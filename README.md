# typr

A local-first flashcard app in the shape of Mochi: write markdown cards, review them on a spaced
schedule, keep every byte on your own disk. Built on [Rune](https://github.com/anomalyco/rune).

Status: early. The v1 design and its task list live in `openspec/changes/typr-v1/`.

## Prerequisites

- Rust `1.99.1` — pinned by `rust-toolchain.toml`, because Rune pins it.
- **Linux only:** development packages `libxkbcommon-dev libwayland-dev` to link, and
  `libxkbcommon-x11-0 mesa-vulkan-drivers` for a windowed run. Without the XKB X11 library every
  windowed start panics with `Library libxkbcommon-x11.so could not be loaded`. Headless
  `--snap` runs need neither.

## Build and run

```sh
cargo run
```

Headless, no window — the same renderer, one frame captured to a PNG:

```sh
cargo run -- --snap /tmp/typr.png
cargo run -q -- --list-scenes
```

Debug builds always accept the headless arguments. A release build needs Rune's `tooling`
feature to take them.

## Dependencies

`rune` and `rune-kit` are pinned to one Rune `dev` revision, and `[patch.crates-io]` repeats the
four crates Rune patches (`winit`, `parley_engine`, `wgpu-hal`, `accesskit_windows`) from that same
revision — a dependency's `[patch]` does not reach this workspace, so without them the build would
silently use stock crates. When Rune's root `[patch.crates-io]` gains an entry, add it here.

`dev` is fast-forward-only, so a revision is a stable target and upgrades are deliberate: bump the
revision, re-read Rune's `[patch.crates-io]`, rebuild.

## Tests

```sh
cargo test
```

The domain layer takes its time from a clock it is given (`clock::Clock`), so "due in ten minutes"
is an assertion rather than a sleep.

## Working on it

```sh
openspec list                       # active changes
openspec instructions apply --change typr-v1 --json
```

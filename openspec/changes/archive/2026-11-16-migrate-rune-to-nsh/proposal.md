# Proposal

## Why

typr's desktop crate is built on Rune, pinned to one git revision. Rune is vendored and patched (`vendor/rune-kit` is 3.0 GB with build output), and every bump re-records the macOS and Linux baselines. Native Shell (NSH, `nsh 0.5.0` installed) draws each screen with the platform's own views and comes with a headless test driver that works through roles and labels. Moving now keeps the work to one crate: `crates/typr-core` (17,639 lines) has no engine dependency, so only `crates/typr` (19,479 lines) changes.

## What Changes

- **BREAKING (build)**: `crates/typr` drops `rune` and `rune-kit` and uses `nsh-core`, `nsh-base`, and `nsh-platform-macos` / `nsh-platform-linux`. The workspace's `[patch.crates-io]` entries for Rune and its `vendor/` copies are removed.
- The app gets an `nsh.json` (Rust mode, `"root": "App"`). `src/main.rs` becomes the `cargo run` desktop runner.
- A new app-owned `ui/` layer on `nsh_base`, with tokens, motion tokens and the core component set, replaces `rune_kit::{Kit, Button, Dialog, Markdown, CodeFrame}` and `rune::tok`. Every screen is rebuilt from it: sidebar, library and tiles, editor, study, history, settings, keymap panel and palette, embeds, and diagnostics.
- Rune's view-per-state model becomes NSH components with signals and bindings. The app state moves into stores that only their own methods write.
- **BREAKING (tooling)**: the `--snap`, `--scene`, `--list-scenes`, `--script` and `--video` arguments are removed. Flow tests move from `rune::{Runtime, Env}` to `nsh_testing::Headless`. Pixel baselines move from `tests/visual.rs` to `nsh test --target macos|linux` screenshot tests (`*.test.tsx`), in light and dark.
- Supported targets: macOS and Linux (GTK 4.14+). Windows and mobile are out of scope.
- The work lands on a git worktree branch. `main` stays on Rune until the branch reaches parity and merges.
- No change to the library format, the schema, or any typr-core behaviour.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `project-structure`: the toolkit is NSH rather than Rune. The single-revision rule now pins every `nsh-*` crate to one source, and the desktop-crate check now uses the app's screens instead of the Rune scene snapshots.
- `visual-language`: tokens come from the app's NSH theme and `ui` layer, and the baseline requirement is restated for platform screenshot tests taken in both appearances.

The other screen capabilities (`card-editor`, `card-grid`, `card-library`, `deck-views`, `diagnostics`, `keymap-panel`, `preferences`, `sidebar`, `study-session`, `card-embeds`, `transfer`, `review-scheduling`) keep their requirements unchanged. The port must keep every scenario they hold passing.

## Impact

- Code: everything under `crates/typr/src` and `crates/typr/tests`, the root `Cargo.toml` and `Cargo.lock`, `vendor/`, and `scripts/vendor-*`.
- CI: `.github/workflows/ci.yml` stops installing Vulkan and Rune's packages, installs GTK 4 dev packages, and runs `cargo test --locked` plus `nsh test --target linux`. The baseline recording flow is redone for NSH screenshots.
- Docs: `README.md` (build, run, prerequisites, headless arguments) and `crates/typr/tests/baselines/README.md`.
- Toolchain: `rust-toolchain.toml` is pinned to 1.99.1 "because Rune pins it". That reason goes away; the pin stays unless NSH needs another version.

---
name: run-typr
description: Run, launch, drive and screenshot the typr NSH desktop app (library, editor, settings, light and dark) on macOS. Use to see a UI change working in the real app, or to compare before/after screenshots.
---

typr is a Rust NSH app. The driver is `nsh test nsh.json --target macos` with steps on stdin; paths are
relative to the repo root. Written on macOS with `nsh` at /opt/homebrew/bin/nsh.

## Run (agent path)

```sh
sh .claude/skills/run-typr/shoot.sh /tmp/shots     # seeds a library, writes six PNGs
```

It opens a real window on the user's screen for a few seconds. Look at the PNGs; a blank frame is a failure.

Before/after check: `git stash -u`, run it into another dir, `git stash pop`, then `cmp` the PNGs.

To drive other flows, copy the `steps.js` the script writes and pipe your own:
`TYPR_DATA_DIR=<lib> nsh test nsh.json --target macos < steps.js`. Calls that worked: `app.waitFor`,
`app.press("role:button[gato]")`, `app.press("Settings")`, `app.keyboard.press("escape")`,
`app.setColorScheme("dark")`, `app.screenshot(path)`.

## Tests

`cargo clippy -p typr --all-targets` and `cargo test --workspace` (the shell tests are slow).

## Gotchas

- Headless `nsh test nsh.json` refuses screenshots ("needs a real platform"): `--target macos` is required.
- `nsh test .` runs the whole suite, not the steps; pass `nsh.json` as the path to use stdin steps.
- Keys are `app.keyboard.press(..)`, not `app.pressKey`.
- The window is phone-width, so the editor's preview and buttons clip; that is the viewport, not a bug.
- With `TYPR_DATA_DIR` exported, `library.test.tsx` "a library that cannot open" fails; set it per command only.
- A library can only be written through `typr_core::store::Db`, so the seed is a temporary
  `crates/typr/tests/zz_seed.rs` that the script removes on exit.
- `nsh run` is the human path (hot-reload window); useless for scripted checks.

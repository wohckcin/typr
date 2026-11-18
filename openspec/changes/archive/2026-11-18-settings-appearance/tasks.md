# Tasks — settings-appearance

## 1. The route to the engine's appearance (spike, one hour)

- [x] 1.1 Determine which route the app crate can take: the root's `appearance` prop
      (`nsh-core-0.5.1/src/ops.rs:2978`) or `Core::set_appearance` (`nsh-core-0.5.1/src/core/classes.rs:185`).
      Record the answer in design.md's Decision 1.
      Verification: a throwaway `cargo run` that sets `Appearance::Light` at mount draws a light window.
- [x] 1.2 Delete the throwaway once the route is written down.
      Verification: `git status` clean of the spike.

## 2. Theme from Settings

- [x] 2.1 `Theme` gains `System` (`crates/typr-core/src/store/settings.rs:35`), plus its parser (`:146`), its
      human name (`:177`) and its default.
      Verification: the settings round-trip tests gain the value and pass; a library row `theme=system` reads as
      `Theme::System`, not as a refusal.
- [x] 2.2 Add the theme control to the Appearance group (`crates/typr/src/screens/settings.rs:213`), beside
      "Show review counts": System, Light, Dark.
      Verification: `tests/settings.rs` case `the_theme_control_writes_the_stored_theme` reads the new value back
      from `Settings`.
- [x] 2.3 Apply the stored theme at mount, before the first paint, and on change; `System` defers to the platform.
      Verification: `tests/appearance.rs` case `the_first_frame_is_the_stored_theme` renders with `theme=light` and
      reads a light surface colour on the first frame.
- [x] 2.4 Make the setting the window follows: `nsh.json` keeps a valid value and the app overrides it.
      Verification: with `theme=dark` stored on a light desktop, the window draws dark; documented in `README.md`'s
      settings table.

## 3. The interface scale leaves the promise

- [x] 3.1 Remove the interface scale from `openspec/specs/preferences/spec.md` and the README's settings table.
      Verification: `grep -rn 'interface scale' openspec/specs README.md` finds nothing.
- [x] 3.2 Remove the scale from the store, not just from the key enum:
      `Settings.scale` and `Settings::DEFAULT_SCALE` (`crates/typr-core/src/store/mod.rs:253-254`, `:301`, `:318`),
      `SettingKey::Scale` and every match arm that names it (`store/settings.rs:25-27`, `:103`, `:117`, `:131`,
      `:146`, `:161`, `:177`, `:206`, `:226`, `:244`, `:276`, `:303`, `:336`), `SettingValue::Scale(f32)` and its
      two text conversions (`:358`, `:370`, `:381`), and `in_scale` (`:410`).
      Verification: `grep -rnw 'scale\|Scale' crates/typr-core/src/store` finds no settings type, field, key or
      arm; `cargo test -p typr-core` green with the scale round-trip tests deleted.
- [x] 3.3 Decide the stale row: a library written before this change still has a `scale` row in its `setting`
      table. Load ignores it (the key is unknown), so either the loader leaves it alone or `Settings::new` drops
      unknown rows once. Record which in design.md Decision 3.
      Verification: a fixture library with a `scale` row opens, reports no refusal, and keeps or loses the row as
      decided — `crates/typr-core/tests/` case `a_library_with_a_stale_scale_row_opens`.

## 4. The visual checks the change unblocks

- [x] 4.1 With `theme=light` stored, run the Finishing and Design-review checklists on the library, the editor and
      the study session; record each group's verdict.
- [x] 4.2 Run the same screens with reduced motion on; every row that plays is skipped and the state change stays.
- [x] 4.3 Read each screen through the accessibility tree and record names, roles and the announcement order.
      Verification for 4.1–4.3: the findings go to `DESIGN.md`'s appearance section, or to a new change if a group
      fails.

## 5. Gate

- [x] 5.1 Run `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings`,
      `bash scripts/check-tokens.sh` once, at the end.
      Verification: all green.
- [x] 5.2 `DESIGN.md`: the appearance section says the app can be reviewed in both appearances from inside the app.

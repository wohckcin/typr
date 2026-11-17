# Design — settings-appearance

## Context

**What exists.** `crates/typr-core/src/store/settings.rs` owns the whole validation surface: `Theme`
(`:35`), `SettingKey::Scale` (`:131`, `:146`), the human name "Interface scale" (`:177`), the range check
`in_scale` (`:410-412`), and the writer (`:336`). `Settings::new` reads every key through the same validators,
so a hand-edited `theme` already reaches the app as a `Theme` value.

**What does not exist.** No screen writes `theme` or `scale`; no code in `crates/typr/src` reads either. The
Appearance group is:

```rust
.child(section("Appearance", toggle().label("Show review counts") …))   // screens/settings.rs:213-217
```

and appearance comes from `nsh.json` (`"appearance": "system"`, `nsh.json:5`), which the engine reads at launch.

**What the engine offers.** `nsh_core::platform::Appearance { System, Light, Dark }`
(`nsh-core-0.5.1/src/platform.rs:35-43`) and `Core::set_appearance(&mut self, appearance: Appearance)`
(`nsh-core-0.5.1/src/core/classes.rs:185`), which the engine also calls when the platform's appearance changes
(`mount.rs:856`). The app crate does not reference `Core` today, so the first task is the spike named in
Decision 1: which of the two documented routes reaches it — the root's `appearance` prop
(`nsh-core-0.5.1/src/ops.rs:2978`) or a call the root makes on mount.

**The scale.** No interface-scale entry point was found in `nsh-core` 0.5.1 (`grep -rln 'interface_scale\|ui_scale'
nsh-core-0.5.1/src` is empty); what exists is per-element `scale(…)` used for press feedback
(`crates/typr/src/ui/button.rs:152`). So `scale` has no engine path, unlike `theme`.

## Goals

- A reader changes the window's appearance from Settings and sees it at once, in the same session, and after a
  restart.
- The stored value is the one the first frame uses: no frame draws in the wrong appearance.
- The screen and `preferences` agree: every control the spec names exists, and nothing it names is absent.
- The change reuses `Store::set_setting` and the existing `Theme` type; no new settings machinery.

## Non-Goals

- A theme editor, more than the stored themes, or per-library palettes.
- Changing `ui/tokens.rs`'s values.
- Re-deriving `DESIGN.md`'s appearance section, which already states how light and dark are checked.

## Decisions

1. **Theme is applied through the engine's appearance, and the spike settles the route.** The app maps its
   stored `Theme::{Light, Dark}` onto `Appearance::{Light, Dark}` and applies it at mount and on change. The
   two candidate routes are the root's `appearance` prop (`ops.rs:2978`, the value `nsh.json` feeds today) and
   `Core::set_appearance` (`classes.rs:185`); task 1.1 names which one the app crate can call, and the design
   is amended with the answer rather than assuming either. Rejected: restyling the app around a CSS variable —
   the engine already resolves the appearance for every element.

2. **`theme` replaces `nsh.json`'s appearance as the deciding value, and `Theme` gains `System`.** `nsh.json` keeps
   a syntactically valid value and the app overrides it at mount. The setting becomes three-valued — System, Light,
   Dark — so a reader can follow the desktop or pin one appearance; the default stays System, which is what
   `nsh.json` says today and what the app already does.

3. **Scale is amended out of the promise.** `preferences` stops naming an interface scale, and the setting leaves
   the store rather than lying dormant: `Settings.scale` and `DEFAULT_SCALE` (`crates/typr-core/src/store/mod.rs:253-254`,
   `:301`, `:318`), `SettingKey::Scale` with its name, help text, default, get, parse and set arms
   (`store/settings.rs:25-27`, `:103`–`:336`), `SettingValue::Scale` with its two text conversions (`:358`, `:370`,
   `:381`) and `in_scale` (`:410`) all go, with the round-trip tests that carry them. A library written before this
   change keeps its inert `scale` row: the loader ignores a key it no longer knows, and dropping unknown rows on the
   next write is the simpler of the two options task 3.3 decides between. Rejected: applying it. `ui/size.rs` and
   `ui/tokens.rs` are the only place dimensions may come from (`tests/tokens.rs`, `scripts/check-tokens.sh`), so a
   factor would turn every size constant into a signal read at every call site — the whole app's dimension surface
   for a setting nothing in `nsh-core` 0.5.1 exposes. It returns if the engine grows a scale of its own.

4. **The first frame uses the stored value.** The theme is read in the same place the data folder is resolved
   (`app-launch` moves that into the first mount), and applied before the first paint; a test asserts the first
   frame's resolved colours, not the second's.

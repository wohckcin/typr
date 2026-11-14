# Design — visual parity with Mochi

## Context

Every screen today builds with `rune_kit` primitives at default sizes: `ROW_HEIGHT = 44.0` in
`crates/typr/src/library.rs`, `type_::BODY.lh(22.0)` in the editor's source pane, `type_::LABEL` for
captions, `Kit::neutral(true)` from a `static KIT`. Colours come from `tok` (`accent`, `text_secondary`,
`line`, `surface`), radii and shadows from `Kit`.

The kit already carries `Palette`, `elevation`, `ty` (type scale), `radius` and `motion`, and every
instance is built from `Kit::neutral(dark)`. That means the material for a token set exists; what is
missing is a module that names the choices so a screen cannot invent a number.

Visual proof exists: eleven scenes snap through `--snap` (`library-5000`, `library-archived`,
`library-trash`, `library-empty`, `library-nested`, `editor`, `editor-leave`, `editor-backlinks`,
`history`, `session`, `session-new`, `session-cram`, `summary`, `settings`, `failure`), and a snapshot is
a PNG on disk. Nothing compares them today, so a regression is invisible.

## Goals / Non-Goals

**Goals**

- One place that decides spacing, type, surfaces, empty states and the review canvas.
- A card that reads as an object, and a review that reads as one thing at a time.
- A committed baseline per scene, so parity is enforced rather than described.

**Non-Goals**

- Copying Mochi's exact palette, iconography or motion. The goal is the same reading experience, not the
  same pixels; where they differ, the repo's own tokens win.
- Custom fonts. The kit's bundled families stay until a licence review says otherwise.
- Animating the reveal. A review is a reading task, and motion in the reveal slows it.

## Decisions

**A `design` module with named tokens and a handful of constructors.** `space(n)`, `type_body()`,
`type_title()`, `surface_card()`, `page_header(title, actions)`, `empty_state(title, line, action)`,
`review_canvas(card)`. Screens then express intent ("a card surface") rather than numbers, and the
"no literal in a screen" requirement is checkable by grep in CI.

**A card is one bordered, elevated surface; metadata is secondary text inside it.** The alternative,
separate name and preview rows, is what makes the current list look like a table.

**The review card keeps its bounds across the reveal.** The answer slot is reserved at the height of the
revealed content measured on the previous card, and a shorter answer pads rather than collapses; that is
what "revealing does not move the card" means in practice. Measuring after the fact and animating the
change was rejected: a jump, however smooth, is a jump.

**Empty states are constructors, not per-screen prose.** One function, one shape: what is missing, why,
and one action. The queue's empty state offers a cram session because that is the only useful thing to do
with nothing due.

**Baselines are committed PNGs compared with a per-channel tolerance.** Scenes are deterministic (no
clock in the view path, `RUNE_DIRS_ROOT` pointed at a fixture), so an exact comparison is nearly
achievable; a small tolerance absorbs driver differences across machines. A missing baseline fails, which
is how a new scene is forced to justify its appearance.

**The comparison lives in a test, not in CI YAML.** `crates/typr/tests/visual.rs` renders each scene
through the same tooling path CI uses and diffs against `tests/baselines/`, so a developer gets the same
verdict locally. CI only runs it.

## Risks / Trade-offs

- Font shaping and GPU differences make pixel comparison brittle across machines. Tolerance plus a
  baseline generated on the CI image (not a laptop) is the mitigation, and the task says which machine
  produced them.
- A large visual change resets many baselines at once, which hides a regression in the noise. The task
  requires baselines be regenerated per commit with the diff read, not in bulk.
- Tokenising the screens touches every view file; the compiler and the baseline diff are the checklist.
- Parity claims are judgements. The design says what is measured (bounds, tokens, groups, empty states)
  and leaves "looks like Mochi" to a human reading the snapshots side by side.

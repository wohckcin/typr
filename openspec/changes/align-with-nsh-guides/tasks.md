# Tasks

## 1. Selection is a card identity

- [x] 1.1 Change the shell's selection to `Signal<Option<Id>>` and resolve Enter and Delete from the selected
      id rather than a list position, in `shell.rs` and `screens/library.rs`. Verify: the workspace compiles and
      `cargo test -p typr --test shell --test grid` passes.
- [x] 1.2 Make `grid(tiles, chosen: Signal<Option<Id>>, …)` highlight the tile whose id is chosen, and clear the
      selection when the chosen card leaves the shown list. Verify: `cargo test -p typr --test grid` passes and
      no test constructs a grid with an index.
- [x] 1.3 Add `SelectionFirst` and `SelectionLast` to the keymap table in `typr-core`, bind them to Home and End
      on the library surface, and move the arrow keys to step from the selected id's position in `visible()`.
      Verify: the keymap tests list the two new chords, and `cargo test -p typr-core` passes.
- [x] 1.4 Add the regression test that types a search which shortens the list, presses Enter, and asserts the
      opened card is the one the highlight is on, then repeats with Delete. Verify: the test fails against the
      old index-based selection and passes after 1.1 to 1.3.

## 2. Editor events carry named objects

- [x] 2.1 Give each `EditorEvent` variant named fields and rename the state-named ones to acts
      (`DiagramFromImage` becomes `StartDiagram`), then update the shell's listeners. Verify: `cargo test -p typr
      --test editor --test shell` passes and every listener still writes the same store command.
- [x] 2.2 Split `screens/editor.rs` into `editor/mod.rs`, `editor/diagram.rs` and `editor/fields.rs` with the
      public paths unchanged. Verify: `cargo test -p typr --test editor --test a11y` passes and `wc -l` shows no
      file above about 550 lines.

## 3. Public structs widen safely

- [ ] 3.1 Add `#[non_exhaustive]` to `Item`, `Cover`, `Rect`, `MergedMasks`, `Field`, `Template` and
      `LibraryInfo`, add `Default` for `Item`, and add a constructor for each type that is built from required
      parts (`Item::new`, `Template::new`, `Cover::new`, `Field::new`). Verify: `cargo check --workspace
      --all-targets` passes.
- [ ] 3.2 Replace every struct literal and every `..Default::default()` update of those types, in the app and in
      the integration tests, with the constructor or with `Default` plus field assignment, since
      `#[non_exhaustive]` forbids both forms outside the crate. Verify: `cargo test --workspace --locked` passes
      and a search of the tests for those type names followed by `{` finds none.
- [ ] 3.3 Rename the editor's template field type to `TemplateField`, since `ui::input::Field` is the text
      field and one word must name one concept, and update the app and the tests that import it. Verify:
      `cargo test --workspace --locked` passes and `Field` appears only as the text-field control and its event.
- [ ] 3.4 Mark `Values` `#[non_exhaustive]` and keep `Values::new` as its only constructor, with a doc comment
      saying why it has no `Default` (its signals belong to the caller's owner). Verify: `cargo test -p typr
      --test settings --test appearance` passes.

## 4. One size per control

- [ ] 4.1 Add a `Size` prop enum with `Sm`, `Md` and `Lg`, and a `metrics()` that returns height, horizontal
      padding, text size and radius, then give `Button`, `Field`, `SubmitField` and `Tile` a `size` prop that
      uses it. Verify: `cargo test -p typr --test button --test grid` passes and the default size is 32 points.
- [ ] 4.2 Replace fixed control heights with minimum heights so a larger text size does not clip, and move any
      remaining control geometry into one documented constants block. Verify: a test renders a button at a raised
      text size and asserts its frame grew; `cargo test -p typr --test appearance` passes.
- [ ] 4.3 Give `Field` a `label` prop drawn above the input, and use it on the settings daily-cap field.
      Verify: `cargo test -p typr --test settings` waits for the label text, and the appearance test still
      renders both themes.

## 5. Hover and press states

- [ ] 5.1 Add hover and press states to `Button` and `Tile` from the motion tokens, with press stronger than
      hover and both suppressed when disabled and when `reduce_motion()` is true. Verify: a headless test hovers
      and presses a button and asserts the resolved background changes, and a test with reduced motion asserts
      the states still differ.
- [ ] 5.2 Add the same hover treatment to `ListRow` where it is missing, keeping the row's selected fill
      stronger than hover. Verify: `cargo test -p typr --test sidebar --test appearance` passes.

## 6. Motion inventory and popup surface

- [ ] 6.1 Build the sidebar's moving selection from the motion tokens and gate it on `reduce_motion()`, or
      rewrite that inventory line as instant with a reason if it does not land cleanly. Verify: the line in
      `DESIGN.md` matches what the running app does in `nsh test desktop.test.tsx`.
- [ ] 6.2 Rewrite the tile wave and skeleton cross-fade lines in `DESIGN.md` as no motion with their reasons, and
      mark which inventory lines were checked in the running app. Verify: every section of `DESIGN.md` states a
      tool and token or a reason, and no line promises motion the code does not have.
- [ ] 6.3 Paint the popup surface in `ui/overlay.rs` with the theme's popover background, border and radius.
      Verify: `cargo test -p typr --test appearance` passes and a search for the card token inside
      `overlay.rs` finds none.

## 7. Command copy

- [ ] 7.1 End the labels of the three dialog-opening editor commands with an ellipsis character. Verify:
      `cargo test -p typr --test editor` finds the labels with the ellipsis, and a test asserts Save has none.

## 8. The seams the review left open

- [ ] 8.1 Split the shell's writer into `apply` and `quiet_apply`, with the slow-command name taken from the
      command instead of passed by the caller. Verify: `cargo test -p typr --test diagnostics --test shell`
      passes and no call site passes a command name.
- [ ] 8.2 Report a refused grade from the study screen through an event or a `Result` rather than a `bool`
      return, and update the shell's handler. Verify: `cargo test -p typr --test study --test shell` passes and
      the refusal path still leaves the card ungraded.
- [ ] 8.3 Move the tile-name flattening out of `screens/grid.rs` into `typr-core`'s markdown module. Verify:
      `cargo test -p typr-core --test markdown` and `cargo test -p typr --test grid` pass.

## Workflow follow-up

- Run `openspec validate align-with-nsh-guides --strict` after the last task.
- When the change is complete, archive it with `openspec archive align-with-nsh-guides` after the specs are
  synced.

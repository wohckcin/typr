# Tasks — native-elements

## 1. The palette and the shortcut panel

- [x] 1.1 Map the surface's bindings to one `CommandItem` per command: `id` the command's name, `label` the
      command's noun, `keywords` every chord it holds. Nothing else becomes an item.
      Verification: `tests/keymap.rs` case `every_command_is_one_item_and_nothing_else` compares the counts both
      ways and finds every chord among the row's keywords.
- [x] 1.2 Mount what is being replaced. Neither `keymap_panel` nor `palette` has a caller today
      (`grep -rn 'keymap_panel\|palette(' crates/typr/src` finds only their definitions), so pressing `?` or
      the palette chord opens nothing while `keymap-panel` promises both. Draw the one `Command` in the shell,
      opened by its chord and closed by the same chord or Escape.
      Verification: `keyboard.test.tsx` case `the shortcut panel opens on its key and closes on it again`, and
      `the palette opens on its chord and lists the library's commands`, both green under `nsh test`.
- [x] 1.3 Replace `palette` (`crates/typr/src/screens/keymap.rs:223`) with `nsh_base`'s `Command`, themed from
      `ui/` (row class, the field, the empty line, the footer).
      Verification: `cargo test -p typr --test keymap` green; typing a filter narrows the rows.
- [x] 1.4 Replace `keymap_panel` (`:195`) with the same element, the footer's run action hidden and the search
      field kept — `keymap-panel` requires the panel to be filtered by typing
      (`openspec/specs/keymap-panel/spec.md:54`), so the field stays.
      Verification: typing into the open panel narrows its rows; the list is the same one the palette draws.
- [x] 1.5 Delete `rows`/`visible_rows`/`row_views` (`:154`, `:161`, `:169`) and the Enter/Escape handling they
      carried.
      Verification: `grep -c 'fn row_views' crates/typr/src/screens/keymap.rs` is 0; `screens/keymap.rs` under
      150 lines.
- [x] 1.6 Press Down twice in the palette and Return: the second row's command runs.
      Verification: `command.test.tsx` case `a command runs from the highlighted row`, green on the macOS host
      (`tests/keymap.rs` presses a row instead: a headless run gives the search field no focus).

## 2. The review history

- [x] 2.1 Draw `screens/history.rs`'s records with `table.rs`: phase, grade, when, due as four columns, newest
      first, the count as the header's badge.
      Verification: `tests/history.rs` case `a_record_reads_across_its_columns` reads the four cells of one row.
- [x] 2.2 Keep the empty state ("No reviews yet") and the count.
      Verification: same file, `no_reviews_yet_is_the_whole_screen`.

## 3. The deck tree

- [x] 3.1 Build the sidebar's decks from `tree.rs`: `TreeItem`s by deck path, folding through the element's own
      expansion, the selection reported back.
      Verification: `tests/sidebar.rs` case `the_deck_rows_are_a_tree` walks the tree and finds the nested deck.
- [x] 3.2 Put the due count where the base element allows: trailing content if it has one, otherwise the item's
      label. Record which in design.md's Decision 3.
      Verification: `tests/shell.rs` case `a_parent_deck_counts_the_cards_due_under_it`.
- [x] 3.3 Check the screen keeps no key handling of its own for the decks: a folder folds on a press, and
      the keys are the base element's.
      Verification: `tests/sidebar.rs` case `a_deck_folds_on_its_own_press_and_selects_itself`; no
      chevron is drawn, so one press is one fold.

## 4. The empty state's label

- [x] 4.1 `screens/grid.rs:121-124`: the label becomes `text`; the bare chord goes.
      Verification: the running app's empty deck shows one sentence and one button, and a screenshot is taken.
- [x] 4.2 Confirm the sentence still says what the deck is ("This deck has no cards yet") rather than what to do,
      which the button says.
      Verification: read the two strings together.

## 5. The rule and the exception

- [x] 5.1 `DESIGN.md`: a short "Elements" section listing the base elements a screen builds from, and the
      hand-built list with `Keycap` named and `ui/status.rs:57`'s reason.
      Verification: the section names every base element the screens use and every hand-built one they keep.
- [x] 5.2 Run `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings`,
      `bash scripts/check-tokens.sh` once, at the end.
      Verification: all green, and the token check finds no literal in the new row classes.

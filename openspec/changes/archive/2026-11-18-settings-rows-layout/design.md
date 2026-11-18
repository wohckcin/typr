# Design — settings-rows-layout

## Context

**Today** (`crates/typr/src/screens/settings.rs`): a `view().gap_4().p_4()` of four `section(name, body)` blocks. Each
section is a muted heading over a stacked body: the "New cards a day" field with its label above it and an Apply
button below; `theme_row` (three `list_row`s stacked, added by `settings-appearance`); a `toggle()` labelled "Show
review counts"; the Library group's `path_row`s, two badges and the Import and Export buttons. `Values` carries the
signals (`cap_text`, `cap`, `show_counts`, `theme`, `refusal`); the shell passes its own `theme`.

**The reference** is described in the proposal. Its parts that matter for the structure: one card, uniform rows, a
hairline between rows, label left, control right, a segmented control, a header with a breadcrumb and a search
field.

**Constraints that shape the how.**

- `tests/tokens.rs` refuses a spacing, size or colour literal in a screen, and `scripts/check-tokens.sh` agrees, so
  every dimension below lives in the `ui` layer or comes from a token.
- The `ui` layer wraps `nsh/base` and screens use only it (`tests/tokens.rs` refuses a base import in a screen).
- Existing tests find controls by name (`role:button[Apply]`, `[System]`, `[Show Database in the file manager]`,
  the text `Show review counts`); they must keep passing as written.

## Decisions

1. **The card draws the hairlines, once.** `ui::rows` is a view whose background is the border colour, with a 1 pt
   gap between its children and a rounded clip; each row has the card colour. A row hidden by the search leaves no
   double line and no stray one, which a border on each row could not promise. Rejected: `border_t` on every row
   but the first — a filter can hide the first row.
2. **`setting_row(label, control)`** in `ui` is the row: a flex row, label at the start, control at the end, one
   minimum height from `Size`, padding from the scale. A row wraps its control under the label when the two do not
   fit (`flex_wrap`). **A path is never truncated** (amended while applying): the `preferences` requirement says each
   path is "displayed in full", and truncating collapsed a long path to nothing in a narrow window, which the
   `the_screen_reports_where_the_library_is` test caught. The path keeps its natural width and the reveal button
   wraps under it.
   **The new-card field flexes** (the user asked): its slot is `flex_1`, so it takes the room the row leaves instead of
   a fixed width, and Apply follows it.
3. **`segmented` is its own component, not `tabs`.** `TabRow` is for sections the caller shows a panel for. A
   segmented control is a value picker: a muted track (the hover token), each segment a base button named by its
   label with `selected` set, the chosen one filled with the card colour and the border. It takes
   `options: Vec<(label, value)>`, the value in force and reports the chosen value; the theme and the flag both
   build on it. Press feedback follows `ListRow` (a deeper wash, no scale), and the segment honours reduced motion
   by having none to skip.
4. **"Show review counts" becomes Off / On.** The reference uses the same shape for flags. `ui::toggle` keeps its
   own test and stays in the layer, unused by this screen, rather than being deleted in a change about layout.
   Rejected: keeping the switch beside segmented controls — two idioms for the same kind of setting.
5. **No group headings; the order carries the grouping.** Review rows, then Appearance rows, then Library rows, as the
   reference has none. `SettingKey::group()` stays; nothing in the store changes.
6. **The search is a signal on `Values`.** `Values::query: Signal<String>`, created by `Values::new`, as `theme` was.
   Each row is wrapped in `when(|| row_matches(label, &query.get()))`, and an empty-state row ("No setting matches")
   shows when none matches. Matching is a case-insensitive substring of the row's label, so a pure function
   (`row_matches`) is unit-tested without the reactive runtime.
7. **The header is a breadcrumb and a field, "General" included.** typr has one page, so the crumb names it and a
   second page later changes one string. The field is `ui::input::field` at `Size::Sm`, with the accessible label
   "Find a setting".
8. **Nothing stretches.** The card spans the content width because the reference does, but every control is sized
   by its content and pinned to the trailing edge, which also settles the "no content max width" finding of the
   1080p review: the wash that spanned 1740 pt belonged to a full-width row, and a segment does not.

## Risks

- **Narrow windows.** A row has a label and a control on one line; at 390 pt the theme's three segments and a label
  may not fit. A row wraps its control under the label when it does not fit (`flex_wrap`), checked in the
  screenshots at 390 pt and 1920 pt, light and dark.
- **Test selectors.** `role:button[Off]` and `[On]` are new names and must be unique on the screen; the paths'
  buttons and the theme's `System` are unaffected.
- **A segmented control inside a row inside a clip.** The focus ring must sit outside the segment's border and not
  be clipped by the card's rounded clip; the card clips only its background fill, and the ring is checked in the
  running app.

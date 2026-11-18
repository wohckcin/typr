# Proposal — settings-rows-layout

## Why

The settings screen (`crates/typr/src/screens/settings.rs`) stacks controls under small group headings, each
control as wide as the window. In the 1920 × 1080 review for `settings-appearance` that read badly: the new-card
field and each theme row's selected wash span about 1740 pt, with the label above the control and nothing
anchoring the two. The reference screenshot the user supplied (a "Settings / General" page of another app) solves
exactly that, and this change copies its structure.

What the reference does, read from the screenshot:

- **One bordered, rounded card** holds every setting. Rows have one height, and one hairline between each pair of
  rows. There are no group headings inside it.
- **Each row is a label on the left and its control on the right.** The label is foreground text; the control sits
  against the trailing edge. A long list stays scannable because both edges are fixed.
- **A choice among a few values is a segmented control**: a muted track, the chosen segment raised with a border
  (System / Light / Dark, Off / On, the text-size steps). A switch is not used.
- **The page has a header**: a breadcrumb ("Settings / General") and a "Find a setting" field at the trailing end.
- **Values that are read, not chosen** (a path) are plain text in the control slot, with a button beside them.

Today's screen differs on each point: grouped sections, stacked label-over-control, a switch for
"Show review counts", list rows for the theme, and no header or search.

## What Changes

- The settings screen draws one card of label-and-control rows, in the order Review, Appearance, Library, with no
  group headings. The hairlines are drawn once, by the card.
- A new `segmented` control in the `ui` layer. Theme (System / Light / Dark) and "Show review counts" (Off / On)
  use it; the theme's list rows and the switch leave the screen.
- A header over the card: the breadcrumb "Settings / General" and a "Find a setting" field that hides the rows
  whose label does not contain what was typed, and says so when none is left.
- Database, Attachments and Log become rows with the path as the value and the reveal button as the control;
  Schema and size become value rows; Import and Export become rows with a button.
- Every existing accessible name stays (`Apply`, `System`, `Light`, `Dark`, `Show Database in the file manager`,
  `Import`, `Export`, the field's label `New cards a day`, and the text `Show review counts`).

Out of scope, on purpose:

- **The sidebar** (the same screenshot shows icons, a status footer and settings sub-pages): its own change,
  `sidebar-restyle`.
- **Settings sub-pages** ("General", "Allowances", …): typr has one page, so the breadcrumb says "General" and the
  structure leaves room for more.
- **Rows typr has no setting for** (accent colour, background image, colourblind mode, fonts, text size, tray, dock):
  none is invented. The reference shows what a row looks like, not which rows exist.
- **Dropdown controls**: no setting here needs one.
- **Changing what any setting stores or validates.**

## Capabilities

### Modified Capabilities

- `preferences`: the settings screen is one card of label-and-control rows with a header that finds a setting by
  name, and a choice among a few values is a segmented control.

## Impact

- `crates/typr/src/screens/settings.rs` (the layout), `crates/typr/src/ui/` (a new `segmented`, a `rows` card and a
  `setting_row`), `crates/typr/src/screens/settings.rs`'s `Values` (a `query` signal).
- `crates/typr/tests/settings.rs`, `tests/a11y.rs`, `tests/shell.rs`, `tests/appearance.rs`: existing cases keep
  passing without edits to what they assert; new cases for the segmented control and the search.
- `ui/toggle.rs` loses its only user on this screen; it stays in the `ui` layer (it has its own test) and is
  noted in the design.
- `DESIGN.md`: the settings row and the appearance review table.

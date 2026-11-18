# Design

Motion inventory for every screen (Design Guides, "Motion inventory"). Tokens are from
`crates/typr/src/ui/motion.rs`. Each line names the tool and the token, or says no motion and why, and
what reduced motion does.

Two kinds of line exist. "Plays" lines are in the running app; they were checked on the macOS host and
by the tests named. "No motion" lines are decisions: the change either has nothing to show or the
movement would cost more than it explains.

## Shell and sidebar

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Sidebar row hover | colour transition, `INSTANT` | instant | plays |
| Group count changes | none: a number updating is not motion | n/a | no motion |
| Sidebar keeps its width | none: the sidebar does not collapse, so there is no layout change to follow | n/a | no motion |
| Current row changes | none: the row's own fill changes in place | n/a | no motion |

No fill slides to the current row. A slide needs each row's measured offset, and the sidebar keeps no
row geometry, so it would add a measurement layer for one transition that a fill change already
explains. The line is revisited if the sidebar measures its rows for another reason.

## Library and card grid

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Tile press | scale to `PRESS_SCALE`, `FAST` | no scale | plays |
| Tile hover | colour wash, `INSTANT`; the card does not move | instant | plays |
| Tiles arrive on page entry | none: a recycled virtual cell replays whatever plays in it, and the library reads in one frame | n/a | no motion |
| Recycled cell or data tick | none: a recycled cell plays nothing | n/a | no motion |
| Empty library | none: the empty state draws at once, because nothing was awaited | n/a | no motion |

## Card editor

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Saved / Unsaved badge | none: the word changes in place; a glyph swap would animate a two-word status | n/a | no motion |
| Picture attached | none: the picture is written into the source text as a reference | n/a | no motion |
| Typing | none: a value that updates per keystroke is not animated | n/a | no motion |

## Study session and review

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Reveal the answer | none: the answer appears in place; the reveal is the result, not a transition | n/a | no motion |
| Grade pressed | scale to `PRESS_SCALE`, `FAST`, through the app's button | no scale | plays |
| Next card | none: the next prompt is set in place | n/a | no motion |
| Session complete | none: the summary draws at once | n/a | no motion |

## History

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Row arrives | none: rows are read in one frame | n/a | no motion |
| Filter change | none: the list is rebuilt in place | n/a | no motion |

## Settings

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Switch toggles | thumb slides, `BASE` (in `ui::toggle`) | instant | plays |
| Section change | none: sections are read top to bottom, not swapped | n/a | no motion |
| Save confirmed | none beyond the control's own state | n/a | no motion |

## Keymap panel and palette

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Panel opens and closes | the popup's own placement; no app motion | n/a | no motion |
| Rows narrow as the user types | none: results update in place | n/a | no motion |

## Embeds and markdown

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Embed block loads its text | none: the block's text is read from the library in one frame, so a skeleton would show for no frame | n/a | no motion |

## Diagnostics and transfer

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Import or export in flight | none: the result line states the outcome when it arrives | n/a | no motion |
| Result line appears | the toast's own entrance and exit from `nsh/base` | the platform's | plays |

## Elements

A surface `nsh/base` already ships is built from it, wrapped in `crates/typr/src/ui` so a screen imports
the app's module and not the base crate (`tests/tokens.rs` refuses `use nsh_base::` in a screen). The
base element owns the keyboard model, the states it reports and what a screen reader hears.

| Screen | Built from | Where |
|---|---|---|
| Command palette | `nsh/base`'s `command` | `ui/command.rs`, drawn for `Mode::Palette` |
| Shortcut panel | the same `command`, with the run action hidden and the field kept, because `keymap-panel` requires a panel filtered by typing | `ui/command.rs`, `Mode::Panel` |
| Review history | `table` with four columns | `ui/table.rs` |
| Sidebar decks | `tree`, folding through the element's own expansion | `ui/tree.rs` |
| Settings toggles and checks | `switch` and `checkbox` | `ui/toggle.rs` |
| Deck and view pickers | `select`, `tabs` | `ui/choice.rs` |
| Card text fields | `input` | `ui/input.rs` |
| Card source, embeds | `editor`, `text_view` | `ui/source.rs`, `ui/prose` |
| Dialogs, menus, popovers, toasts | `dialog`, `menu`, `popover`, `toast` | `ui/overlay.rs`, `ui/status.rs` |
| Buttons and tiles | `button` under the app's props | `ui/button.rs` |

Hand-built, and why:

| Element | Why it is not a base element |
|---|---|
| `Keycap` (`ui/status.rs:57`) | The base crate has `keys.rs`, which reads and binds keystrokes but draws no keycap: a chord is a bordered name with the app's radius and type. |
| `list_row` (`ui/choice.rs`) | The base `listbox` is a selection model; a sidebar row is a label with a chosen state the app owns. |
| `badge`, `status_line` | A count and a state are text with the app's tokens, not controls. |

## Appearance, without pictures

Appearance is checked by the styles a screen resolves to, not by captured images. `tests/appearance.rs`
renders every screen in the light and the dark appearance and reads `screen().span_style(..)`: the same
text keeps its font size and weight, and its colour is the theme's for that appearance, so the two
differ. The library is read through `typr::shell::shell`, the same function the app draws it with, so
there is one library screen and the pass cannot be aimed at a copy. `tests/tokens.rs` refuses a
spacing, type, radius or colour literal in a screen.

The app can be reviewed in both appearances from inside it: Settings → Appearance → Theme offers System,
Light and Dark, stores the choice in the library's `theme` setting, and the window follows that setting
rather than `nsh.json`'s `appearance`. `the_first_frame_is_the_stored_theme` proves the first frame already
draws in the stored theme.

### Review on the macOS host, 1920 × 1080 pt, light and dark

Run for `settings-appearance` with `nsh test nsh.json --target macos` and `app.setViewport(1920, 1080)`, on the
library, the editor, the study session and settings, with `theme=light` and `theme=dark` stored (so the stored
theme, not the desktop's, drew each). The host's default window is phone-sized; the app's own window is
960 × 640 (`bin/typr_nsh.rs`).

| Group | Verdict |
|---|---|
| Colour and surfaces | passed: tokens only, both appearances balanced, the stored theme wins over the desktop's, a change in Settings repaints the whole window in the same frame |
| Radius, typography, spacing | passed on the four screens; no mid-word breaks at 1080p |
| Theme control | fixed: side by side it left the window and broke labels mid-word at 390 pt, so it is stacked |
| Reduced motion | fixed: the toggle thumb slid regardless; it now jumps. Button and tile press already branched |
| Alignment, layout | **failed, not fixed (UI rework pending)**: the editor's preview is an unlabelled strip beside a 860 pt source column with ~800 pt empty, and Save and Attach stretch to that column; settings has no content max width, so the cap field and each theme row's selected wash span ~1740 pt; study sits in the top-left of an empty window |
| Minimum window size | **failed, not fixed**: at 390 pt the editor's columns, the study buttons ("Remembered") and settings' paths and toggle label clip |
| Hover, pressed, focus rings | not checked in pictures (a still shows none); `desktop.test.tsx` covers hover and press |
| Sidebar footer (`sidebar-restyle`) | passed at 1920 × 1080 and 390 × 600, light and dark: the state row is the foot of the column under one hairline, the dot is visible on both backgrounds, nothing is pushed out of the short column; the dot is a second cue beside the text and the tone |
| Settings rows (`settings-rows-layout`) | passed at 1728 × 1080, light and dark, against the reference: breadcrumb and "Find a setting" over one card, one label-and-control row each, labels on one spine and controls on the trailing one, single hairlines drawn by the card, the new-card field flexing across its row, segmented Theme and Off / On. At the app's own 960 × 640 the rows wrap the control under the label and nothing clips except a path wider than the card |
| Settings, long paths | **known limit, not fixed**: a path longer than the card is clipped at its edge, because it is one unbreakable word and the text API has no break-anywhere property; a real path (`~/Library/Application Support/…/typr.sqlite3`, about 55 characters) fits at 960. Truncating was rejected: it hid the whole path in a narrow window |
| Settings at 390 pt | **not designed**: the fixed 220 pt sidebar leaves about 170 pt for the card, which no three-segment control fits; the same minimum-window finding as above |
| 1920 pt capture | the host window is capped at this Mac's 1728 pt width, so a 1920 pt viewport crops the right edge; 1728 × 1080 was used |
| Accessibility | names and roles of the three theme choices are asserted in `tests/a11y.rs`; a screen reader, announcement order and focus order on the host were not run |

The hover wash and the press scale are the exception that needs a picture, because the headless driver
keeps no view style and cannot take one. `desktop.test.tsx` hovers a button on the macOS host and asserts
the screenshot changed, and skips that step where the app opens as a phone. The half-point colours and
the state roles are unit-tested in `ui/button.rs`, where they need no reactive runtime.

# Design

Motion inventory for every screen the port touches, written before the screen's code (Design
Guides, "Motion inventory"). Tokens are from `crates/typr/src/ui/motion.rs`. Each line names the
tool and the token, and what reduced motion does. Lines are checked in the running app when the
screen lands (task 4.10).

## Shell and sidebar (task 4.1)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Sidebar row hover | colour transition, `INSTANT` | instant |
| Current deck changes | one selection fill slides to it, `BASE`, ease-in-out | instant |
| Group count changes | none: a number updating is not motion | n/a |
| Sidebar collapses at a narrow width | layout animation, `BASE` | instant |

## Library and card grid (task 4.2)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Tile press | scale to `PRESS_SCALE`, `FAST` | no scale |
| Tile hover | colour wash only, `INSTANT`; the card does not move | instant |
| Tiles arrive on page entry | wave, rows fade in `RISE` away, `SLOWER`, first 10 rows | none |
| Recycled cell or data tick | none (a recycled cell plays nothing) | n/a |
| Empty library | empty state fades in, `BASE` | instant |

## Card editor (task 4.3)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Save state changes (saved to saving) | glyph swap, `BASE` | instant |
| Attachment tile arrives | rise `RISE` and fade, `BASE` | instant |
| Typing | none: a value that updates per keystroke is not animated | n/a |

## Study session and review (task 4.4)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Reveal the answer | answer area fades in inside its fixed bounds, `BASE`; the card does not move | instant |
| Grade pressed | scale to `PRESS_SCALE`, `FAST` | no scale |
| Next card | card cross-fades, `FAST` | instant |
| Session complete | empty state fades in, `BASE` | instant |

## History (task 4.5)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Row arrives | fade, `INSTANT` | instant |
| Filter change | list cross-fades, `FAST` | instant |

## Settings (task 4.6)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Switch toggles | thumb slides, `BASE` (in `ui::toggle`) | instant |
| Section change | cross-fade, `FAST` | instant |
| Save confirmed | none beyond the control's own state: the switch shows the outcome | n/a |

## Keymap panel and palette (task 4.7)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Panel opens | popup grows from its trigger, `SLOW`, slides `SLIDE` | instant |
| Panel closes | fade out, `BASE` | instant |
| Result list changes as the user types | none: results update in place | n/a |

## Embeds and markdown (task 4.8)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Embed block loads its text | skeleton pulses, fades out when content arrives, `BASE` | static skeleton |
| Code block copy confirmed | check glyph swap, `FAST` | instant |

## Diagnostics and transfer (task 4.9)

| Change | Tool, token | Reduced motion |
|---|---|---|
| Progress of an import or export | indeterminate spinner label (no animation; see design.md 3.2) | n/a |
| Result line appears | toast fades in, `BASE` | instant |
| Toast clears | fades out, `FAST`, after `TOAST_MS` | instant |

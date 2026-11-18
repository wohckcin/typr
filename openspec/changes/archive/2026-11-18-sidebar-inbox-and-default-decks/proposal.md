# Proposal

## Why

The sidebar opens on "All cards", so the user has to look through decks to find work. It also lists views, tags
and a Diagnostics row the user rarely needs. A new library starts with one deck named `Default`, which says
nothing about what to put in it. The target layout (the reference screenshot) puts the work first. An Inbox
gathers every card that is due or new, two starter decks (Flashcards and Notes) separate study material from
reference notes, and the column ends with the actions a user reaches for: Import, Settings and Trash.

## What Changes

- The sidebar becomes, top to bottom: a search field with its chord, a **navigation** group (Dashboard, Inbox
  and, nested under Inbox, Due today and New cards), the **Decks** group (the deck tree plus a `+ New Deck`
  row), and a **utilities** group (Import, Settings, Trash). The library state row stays at the foot.
- Every navigation, deck and utility row draws a leading icon, as in the screenshot.
- **Inbox** is the sidebar's default selection instead of "All cards". It shows every studyable card that is
  due now or still has a prompt that was never scheduled. **Due today** narrows it to the due cards, and
  **New cards** narrows it to the unscheduled ones. Each row shows its count.
- **Dashboard** is a visible row that is disabled and marked as coming later. It opens nothing; the dashboard
  itself is out of scope.
- **`+ New Deck`** turns into an inline name field. Enter creates a top-level deck with that name, and Escape
  or an empty name cancels.
- **Import** runs the existing import dialog from the sidebar, and its result appears as a toast.
- **Trash** opens a new Trash screen. It lists trashed cards with Restore per card and an Empty trash action
  that asks for confirmation. The screen uses the store's existing `RestoreCard` and `EmptyTrash` commands.
- **BREAKING (UI):** the Views and Tags groups, the "All cards" row and the Diagnostics row leave the sidebar.
  Diagnostics stays reachable from a link on the Settings screen.
- A library created on first open starts with two top-level decks, **Flashcards** and **Notes**, instead of
  `Default`. Libraries that already have decks are not changed.

Out of scope: the dashboard's content, renaming or deleting decks from the sidebar, a new way to reach saved
views and tags (they remain in the store and the import/export format), and changing how study queues are built.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `sidebar`: new groups and rows (Dashboard, Inbox with Due today and New cards, `+ New Deck`, Import,
  Settings, Trash), leading icons, Inbox as the default selection, a field-shaped search row. The views, tags,
  "All cards" and Diagnostics rows are removed.
- `card-library`: a new library is created with the Flashcards and Notes decks, and the trash gains a screen
  that lists trashed cards with Restore and Empty trash.
- `preferences`: the Settings screen links to the Diagnostics screen.

## Impact

- `crates/typr/src/screens/sidebar.rs`: groups, items, icons, Inbox counts, the new-deck row.
- `crates/typr/src/shell.rs`: routing for the new keys (`inbox`, `inbox:due`, `inbox:new`, `trash`,
  `import`, `new-deck`), the default selection, `Screen::Trash`.
- `crates/typr/src/screens/library.rs`: `cards_in` learns the Inbox selections.
- New `crates/typr/src/screens/trash.rs`, registered in `screens::NAMES` so the appearance check covers it.
- `crates/typr/src/screens/settings.rs`: the Diagnostics link.
- `crates/typr/src/paths.rs`: first-open decks.
- `crates/typr/src/ui/`: an icon element built on `nsh_core`'s `svg`, with inline SVG markup and no new
  dependency.
- Tests: `crates/typr/tests/sidebar.rs`, `shell.rs`, `a11y.rs`, `appearance.rs`, plus new tests for Inbox
  filtering, first-open decks and the Trash screen.

# Design — library-starts-with-a-deck

## Context

`crates/typr/src/paths.rs:37-60`:

```rust
pub fn open_from_environment() -> Result<Db, StoreError> {
    let root = data_root()?;
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(attachments_root(&root))?;
    let db = Db::open(&database_path(&root))?;
    ...
}
```

`Db::open` (`crates/typr-core/src/store/db.rs:62`) creates the file and migrates the schema. Neither it nor the
caller writes a row, so a fresh library is a database with no decks.

The action the empty grid offers, read from `crates/typr/src/shell.rs`:

- `grid(tiles, selection, on_new, open)` (`crates/typr/src/screens/grid.rs:104`) draws
  `button_new(on_new)` when the deck holds no cards.
- Both call sites passed `|| {}` (`shell.rs:626`, `:748`).
- The real action, `new_card` (`shell.rs:405-437`), picks the deck from the sidebar's selection, else the first
  deck of the snapshot, and returns when there is none.

So the button was a no-op twice over: an empty closure, and an action with no deck to act on.

## Goals

- A fresh library is usable on the first frame: a deck exists, and its empty state's action adds a card.
- One action: the button and the `c` chord cannot come to different cards.
- The seed is not repeated: reopening a library that holds a deck writes nothing.

## Non-Goals

- Seeding sample cards, tips, or any other content: the deck is empty and says so.
- A first-run wizard, an onboarding screen, or a prompt for the deck's name.
- Deck deletion. The store has `ArchiveDeck` and no delete, so "no deck at all" means a library that has never
  held one; a library whose decks are archived still holds them and is not re-seeded.

## Decisions

1. **The seed belongs where the app opens the library, not in the store.** `Db::open` is used by tests, imports
   and the transfer paths, all of which mean something specific by an empty library; a seed inside it would
   write a deck behind every caller's back. `open_from_environment` is the one place that means "the app's
   library", so the deck is seeded there.

2. **The condition is "no deck", not "no card", and it is checked once per open.** A library with a deck and no
   card is in the state the empty grid describes, and seeding again there would add a second deck each launch.
   The check is `snapshot.decks.is_empty()` before any write, and the write happens only when it holds.

3. **The seeded deck is called `Default`.** It is a container, not content: the user's first real deck will be
   named for what they study, and a name that describes a role rather than a subject does not have to be
   renamed later.

4. **The empty deck's button runs the new-card action itself.** The shell already owns one closure for "make a
   card" (the chord's), so the grid is handed that closure rather than a second implementation; the alternative
   — teaching `grid` to create cards — would put a store write in a screen and give the app two ways to make a
   card.

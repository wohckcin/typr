# Design — Mochi import and export

## Context

A typr library is a SQLite database at `<data>/typr.sqlite3` with `deck`, `card`, `prompt`, `review_log`,
`tag`, `card_tag`, `editor_draft`, `settings`, `field` and `mask` tables, images in
`<data>/attachments/`, and `PRAGMA user_version` for migrations. Schema facts are reported by
`Db::schema_facts()`.

The store writes through commands in single transactions (`Db::apply`), which is the surface an importer
should use so that every rule the app enforces is enforced on imported data too.

## Goals / Non-Goals

**Goals**

- One file in, one file out, with nothing lost.
- Import that fails cleanly instead of half-writing.
- Text formats usable by other tools without reading typr's schema.

**Non-Goals**

- Anki's `.apkg` (a different schedule model and a media database), unless it later proves cheap.
- Merging into an existing library on a per-card basis with conflict prompts. An import adds; it does
  not reconcile.
- Sync, which stays out of scope for the clone.

## Decisions

**Export is a consistent copy of the database, not a bespoke archive.** `VACUUM INTO <path>` produces one
file with no WAL sidecar, in one statement, without touching the live file. A zip of JSON would need a
serialiser per table and would lose what SQLite already guarantees. The extension is `.typr`, and
opening it is `Db::open`, which already refuses a foreign file with its path named
(`StoreError::Unreadable`).

**Import runs in one transaction through the store's own commands.** `Db::apply` per card keeps template
rendering, sides, prompts and references consistent, and a failure at card forty rolls back the whole
import. The alternative, bulk `INSERT`s, would let an imported card exist with a stale `search_text`.

**Mochi's history maps per variant, and grades map by meaning.** Mochi records `remembered` and
`forgot` per review variant, which is exactly typr's binary grade; each variant becomes a prompt key
(`fwd`, `rev`, `cloze:<n>`, `mask:<n>`), and its records are written with the times Mochi stored. A
card whose imported records end before today keeps the due date its last record produced, so the
library arrives where the user left it.

**The importer reads the shape before writing.** A Mochi export is a zip whose manifest names the format
version and the tables it holds. Reading the manifest first, then refusing an unknown version, is what
makes "refused whole" possible; a streaming reader that starts writing on the first entry cannot promise
it.

**One new dependency at most, in the backend.** A zip reader that is pure Rust (no C, no desktop stack)
keeps the `project-structure` invariant intact; it is recorded in `Cargo.toml` with a comment saying
why, the same way the four Rune patches are documented at the workspace root.

**Text formats carry the syntax, not the schedule.** CSV columns are `deck, sides, tags, fields,
markdown`; Markdown uses one file per deck with `#` deck headings and cards separated by a blank line
pair, keeping `---` and `{{…}}` verbatim so a round trip reproduces prompts rather than approximating
them.

## Risks / Trade-offs

- Mochi's export shape is a moving target; the manifest version check is the only protection, and a
  wrong guess must fail loudly rather than import nonsense. The importer therefore validates counts
  (cards in the manifest versus cards written) and reports a mismatch.
- Importing a large library through one transaction holds a write lock for its duration. That is the
  price of all-or-nothing, and the UI must run it off the frame thread, which the store's worker
  already provides.
- `VACUUM INTO` needs free disk space equal to the library; a failure is reported rather than truncated.
- CSV has no type system, so fields arrive as strings, which is what they are.

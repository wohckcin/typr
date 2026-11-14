//! The schema, and the only code that changes it.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::StoreError;

/// What a library recorded about its schema before it was opened, and what it records now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFacts {
    /// The `user_version` the file held before migration.
    pub found: i64,
    /// The `user_version` it holds afterwards.
    pub written: i64,
    /// How many migration steps ran.
    pub applied: usize,
}

/// One SQL script per version, applied in order. `PRAGMA user_version` records how many ran.
pub const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12];

/// The step that adds the name column, whose backfill needs more than SQL.
const NAMES: usize = 9;

/// Deck views: how a deck is read, and the order its cards are read in.
///
/// The view is a column rather than a settings row because it belongs to a deck: a deck of long
/// cards wants the notebook while a deck of vocabulary wants the list. `NULL` reads as the grid,
/// which is the only view a version-11 library had, so nothing is rewritten on upgrade.
///
/// `card.position` is the user's order, which is why it is not `updated_at`: reordering is a
/// statement about the deck, not about when a card was last touched. Every existing card starts at
/// zero and a deck with every card at zero reads in the order it read before.
const V12: &str = r#"
ALTER TABLE deck ADD COLUMN view TEXT;
ALTER TABLE card ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
CREATE INDEX card_position ON card (deck_id, position, id);
"#;

/// FSRS scheduling: what the model remembers about a prompt, and the three settings that choose it.
///
/// The memory columns are nullable because a prompt graded by the fixed scheduler has no memory,
/// and the first FSRS grade bootstraps one from the interval it already reached. The settings rows
/// are written here so a fresh library and a migrated one read the same values back; a missing row
/// still reads as the default, so a hand-edited file cannot make the scheduler unknown.
///
/// The weights text duplicates `crate::fsrs::DEFAULT_WEIGHTS` because a migration is one SQL string
/// and `concat!` takes literals only; a test pins the two together.
const V11: &str = r#"
ALTER TABLE prompt ADD COLUMN stability REAL;
ALTER TABLE prompt ADD COLUMN difficulty REAL;
INSERT OR IGNORE INTO settings (key, value) VALUES ('scheduler', 'fixed');
INSERT OR IGNORE INTO settings (key, value) VALUES ('retention', '90');
INSERT OR IGNORE INTO settings (key, value) VALUES ('fsrs_weights', '0.40255,1.18385,3.173,15.69105,7.1949,0.5345,1.4604,0.0046,1.54575,0.1192,1.01925,1.9395,0.11,0.29605,2.2698,0.2315,2.9898,0.51655,0.6621');
"#;

/// Card names: the line a list shows and a `[[reference]]` resolves against.
///
/// The name is a column rather than a read-time preview because a reference resolves against it
/// with one index lookup, and because the name a reference points at must not move when the card's
/// fields change around it. `name_template` is the card's own name template; `NULL` means "name me
/// from my first side".
///
/// Existing cards are backfilled in Rust, in `backfill_names`, not by a `substr` here: a first side
/// ends at a `---` outside a fence, which is a rule `crate::sides` owns and SQL cannot repeat.
const V9: &str = r#"
ALTER TABLE card ADD COLUMN name TEXT NOT NULL DEFAULT '';
ALTER TABLE card ADD COLUMN name_template TEXT;
CREATE INDEX card_name ON card (name COLLATE NOCASE);
"#;

/// References between cards: a `[[name]]` in one card's text that names another.
///
/// A row is written when the referring card is saved and dropped when it no longer says that name,
/// which is what makes a backlink a join rather than a scan of every card's text. Both sides
/// cascade, so deleting a card takes the references it made and the ones made to it.
///
/// `reference_to` is the direction a card's own backlinks are read in; the primary key already
/// serves the direction its outgoing references are read in.
const V10: &str = r#"
CREATE TABLE reference (
    from_card INTEGER NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    to_card   INTEGER NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    PRIMARY KEY (from_card, to_card)
);
CREATE INDEX reference_to ON reference (to_card);
"#;

/// Custom views: a pin on a card, and the filters the user saved.
///
/// `view.deck` is a plain integer rather than a foreign key: a view whose deck is deleted keeps its
/// id, and the library reads a scope no deck carries as the whole library. That is a deliberate
/// choice of "show everything and say why" over "silently empty".
///
/// Tags are one string joined by ASCII unit separator, which the tag editor cannot produce: a tag
/// is trimmed text, and a set of them has no other column to live in.
const V8: &str = r#"
ALTER TABLE card ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
CREATE TABLE view (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    deck        INTEGER,
    query       TEXT NOT NULL,
    tags        TEXT NOT NULL,
    mode        TEXT NOT NULL,
    pinned_only INTEGER NOT NULL,
    sort_key    TEXT NOT NULL,
    sort_dir    TEXT NOT NULL
);
"#;

/// Card templates: a card's text can be rendered from a template and a set of named fields.
///
/// The template lives in two places because it is inherited: a card may carry its own, a deck may
/// carry a default for its cards, and the nearest one wins. `NULL` in both means "nothing above me
/// sets one", which is how a library with no templates keeps behaving exactly as it did.
///
/// Fields are a table rather than a column of JSON: the next roadmap row asks "which cards have an
/// Example" for custom views, and that is a `WHERE` clause here rather than a parser in Rust.
const V7: &str = r#"
ALTER TABLE card ADD COLUMN template TEXT;
ALTER TABLE deck ADD COLUMN template TEXT;
CREATE TABLE field (
    card_id INTEGER NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    name    TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (card_id, name)
);
"#;

/// Diagram cards: a card may carry an image instead of markdown, and each group of rectangles drawn
/// on it is a review of its own.
///
/// The view is V5's body again, widened by one disjunct that admits a card holding a mask. V5 is
/// key-aware and knows `rev`, so copying its body verbatim is what keeps reverse working; restating
/// the view from the V3 baseline would drop both the hidden-text and the reverse clauses.
const V6: &str = r#"
ALTER TABLE card ADD COLUMN kind TEXT NOT NULL DEFAULT 'text';
ALTER TABLE card ADD COLUMN image TEXT;
ALTER TABLE card ADD COLUMN caption TEXT;
CREATE TABLE mask (
    card_id INTEGER NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    group_no INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    w REAL NOT NULL,
    h REAL NOT NULL,
    PRIMARY KEY (card_id, group_no, x, y, w, h)
);
CREATE INDEX mask_group ON mask (card_id, group_no);
DROP VIEW IF EXISTS studyable_prompt;
CREATE VIEW studyable_prompt AS
WITH RECURSIVE archived_deck(id) AS (
    SELECT id FROM deck WHERE archived = 1
    UNION
    SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
),
deck_reverse(id, inherited) AS (
    SELECT id, COALESCE(reverse, (SELECT COALESCE(CAST(value AS INTEGER), 0) FROM settings WHERE key = 'reverse_default'))
    FROM deck WHERE parent_id IS NULL
    UNION
    SELECT d.id, COALESCE(d.reverse, r.inherited)
    FROM deck d JOIN deck_reverse r ON d.parent_id = r.id
)
SELECT p.card_id, p.key, p.phase, p.due_at, p.interval_ms
FROM prompt p
JOIN card c ON c.id = p.card_id
WHERE c.trashed_at IS NULL
  AND c.archived = 0
  AND p.retired = 0
  AND (
        (p.key = 'fwd' AND c.sides >= 2)
        OR (p.key LIKE 'cloze:%' AND c.has_cloze = 1)
        OR (
            p.key = 'rev'
            AND c.sides >= 2
            AND COALESCE(
                    c.reverse,
                    (SELECT inherited FROM deck_reverse WHERE id = c.deck_id),
                    0
                ) = 1
        )
        OR EXISTS (SELECT 1 FROM mask m WHERE m.card_id = c.id)
    )
  AND c.deck_id NOT IN (SELECT id FROM archived_deck);
"#;

/// Reverse reviews: a `rev` prompt queues only where the card, its nearest deck or the library
/// default turns reverse on, and NULL means "inherit".
///
/// The view is V4's body again, widened by one disjunct. V4 is key-aware, so `rev` is admitted by
/// nothing until this clause names it, and no exclusion filter is needed. `deck_reverse` walks in
/// the opposite direction to `archived_deck` — from the roots down — because a value is inherited
/// downwards; a deck no root reaches has no row, so `COALESCE(…, 0)` reads it as off rather than
/// as the library default.
const V5: &str = r#"
ALTER TABLE deck ADD COLUMN reverse INTEGER;
ALTER TABLE card ADD COLUMN reverse INTEGER;
INSERT OR IGNORE INTO settings (key, value) VALUES ('reverse_default', '0');
-- Every two-sided card claims a reverse prompt from now on, so a library written before this
-- column existed gets the row here rather than waiting to be saved again: turning reverse on must
-- queue what is already there.
INSERT OR IGNORE INTO prompt (card_id, key, phase)
    SELECT id, 'rev', 'learn' FROM card WHERE sides >= 2;
DROP VIEW IF EXISTS studyable_prompt;
CREATE VIEW studyable_prompt AS
WITH RECURSIVE archived_deck(id) AS (
    SELECT id FROM deck WHERE archived = 1
    UNION
    SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
),
deck_reverse(id, inherited) AS (
    SELECT id, COALESCE(reverse, (SELECT COALESCE(CAST(value AS INTEGER), 0) FROM settings WHERE key = 'reverse_default'))
    FROM deck WHERE parent_id IS NULL
    UNION
    SELECT d.id, COALESCE(d.reverse, r.inherited)
    FROM deck d JOIN deck_reverse r ON d.parent_id = r.id
)
SELECT p.card_id, p.key, p.phase, p.due_at, p.interval_ms
FROM prompt p
JOIN card c ON c.id = p.card_id
WHERE c.trashed_at IS NULL
  AND c.archived = 0
  AND p.retired = 0
  AND (
        (p.key = 'fwd' AND c.sides >= 2)
        OR (p.key LIKE 'cloze:%' AND c.has_cloze = 1)
        OR (
            p.key = 'rev'
            AND c.sides >= 2
            AND COALESCE(
                    c.reverse,
                    (SELECT inherited FROM deck_reverse WHERE id = c.deck_id),
                    0
                ) = 1
        )
    )
  AND c.deck_id NOT IN (SELECT id FROM archived_deck);
"#;

/// Hidden text makes a one-sided card studyable, so the view's last predicate stops resting on sides
/// alone. Everything else about the view is V3's: a trashed card, an archived card and a card under an
/// archived deck stay out of the queues, and `DROP VIEW IF EXISTS` keeps a file whose view is already
/// gone opening cleanly.
const V4: &str = r#"
-- No backfill: a card's hidden-text flag is only meaningful for prompts this schema introduced,
-- and a plain `{{` search would also match a fence or a code span, which the parser reads as
-- text. Every library that reaches this step holds only `fwd` prompts, which the view gates on
-- sides below.
ALTER TABLE card ADD COLUMN has_cloze INTEGER NOT NULL DEFAULT 0;
ALTER TABLE prompt ADD COLUMN retired INTEGER NOT NULL DEFAULT 0;
DROP VIEW IF EXISTS studyable_prompt;
CREATE VIEW studyable_prompt AS
WITH RECURSIVE archived_deck(id) AS (
    SELECT id FROM deck WHERE archived = 1
    UNION
    SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
)
SELECT p.card_id, p.key, p.phase, p.due_at, p.interval_ms
FROM prompt p
JOIN card c ON c.id = p.card_id
WHERE c.trashed_at IS NULL
  AND c.archived = 0
  AND p.retired = 0
  AND (
        (p.key = 'fwd' AND c.sides >= 2)
        OR (p.key LIKE 'cloze:%' AND c.has_cloze = 1)
    )
  AND c.deck_id NOT IN (SELECT id FROM archived_deck);
"#;

/// The one definition of "a prompt a session may offer": the card is not trashed or archived, it
/// has at least two sides, and no deck above it is archived. Both queue queries read this view, so
/// the rule cannot drift between them.
const V3: &str = r#"
CREATE VIEW studyable_prompt AS
WITH RECURSIVE archived_deck(id) AS (
    SELECT id FROM deck WHERE archived = 1
    UNION
    SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
)
SELECT p.card_id, p.key, p.phase, p.due_at, p.interval_ms
FROM prompt p
JOIN card c ON c.id = p.card_id
WHERE c.trashed_at IS NULL
  AND c.archived = 0
  AND c.sides >= 2
  AND c.deck_id NOT IN (SELECT id FROM archived_deck);
"#;

/// V3's view, for tests that put a file back to the schema a version-3 build left.
#[cfg(test)]
pub(crate) const V3_SQL: &str = V3;

/// V4's view, for tests that put a file back to the schema a version-4 build left. Quoted out of
/// [`V4`] rather than copied, so the two cannot drift apart.
#[cfg(test)]
pub(crate) fn v4_view_sql() -> &'static str {
    const MARKER: &str = "DROP VIEW IF EXISTS studyable_prompt;";
    let at = V4.find(MARKER).expect("V4 drops the view") + MARKER.len();
    &V4[at..]
}

/// V5's view, for tests that put a file back to the schema a version-5 build left. Quoted out of
/// [`V5`] rather than copied, so the two cannot drift apart.
#[cfg(test)]
pub(crate) fn v5_view_sql() -> &'static str {
    const MARKER: &str = "DROP VIEW IF EXISTS studyable_prompt;";
    let at = V5.find(MARKER).expect("V5 drops the view") + MARKER.len();
    &V5[at..]
}

/// Drafts belong to a card, or to the card that has not been saved yet, so the key needs no
/// foreign key: `0` stands for the new card.
const V2: &str = r#"
CREATE TABLE editor_draft_v2 (
    card_id    INTEGER PRIMARY KEY,
    text       TEXT    NOT NULL,
    updated_at INTEGER NOT NULL
);
INSERT INTO editor_draft_v2 (card_id, text, updated_at)
    SELECT card_id, text, updated_at FROM editor_draft;
DROP TABLE editor_draft;
ALTER TABLE editor_draft_v2 RENAME TO editor_draft;
"#;

const V1: &str = r#"
CREATE TABLE deck (
    id        INTEGER PRIMARY KEY,
    parent_id INTEGER REFERENCES deck(id) ON DELETE CASCADE,
    name      TEXT    NOT NULL,
    archived  INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX deck_name ON deck(IFNULL(parent_id, -1), name COLLATE NOCASE);

CREATE TABLE card (
    id          INTEGER PRIMARY KEY,
    deck_id     INTEGER NOT NULL REFERENCES deck(id) ON DELETE CASCADE,
    markdown    TEXT    NOT NULL,
    sides       INTEGER NOT NULL DEFAULT 1,
    archived    INTEGER NOT NULL DEFAULT 0,
    trashed_at  INTEGER,
    search_text TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX card_deck ON card(deck_id);

CREATE TABLE prompt (
    card_id     INTEGER NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    key         TEXT    NOT NULL,
    phase       TEXT    NOT NULL CHECK (phase IN ('learn', 'review', 'relearn')),
    due_at      INTEGER,
    interval_ms INTEGER NOT NULL DEFAULT 0,
    reps        INTEGER NOT NULL DEFAULT 0,
    lapses      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (card_id, key)
);
CREATE INDEX prompt_due ON prompt(due_at) WHERE due_at IS NOT NULL;

CREATE TABLE tag (
    id   INTEGER PRIMARY KEY,
    -- One row per tag, in the casing it was first written: matching is case-insensitive, display
    -- is not.
    name TEXT NOT NULL COLLATE NOCASE UNIQUE
);
CREATE TABLE card_tag (
    card_id INTEGER NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, tag_id)
);

CREATE TABLE review_log (
    card_id     INTEGER NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    key         TEXT    NOT NULL,
    phase       TEXT    NOT NULL CHECK (phase IN ('learn', 'review', 'relearn')),
    at          INTEGER NOT NULL,
    grade       TEXT    NOT NULL CHECK (grade IN ('forgot', 'remembered')),
    interval_ms INTEGER NOT NULL,
    due_at      INTEGER,
    duration_ms INTEGER
);
CREATE INDEX review_log_card ON review_log(card_id, key, at);
CREATE INDEX review_log_at ON review_log(at);

CREATE TABLE editor_draft (
    card_id    INTEGER PRIMARY KEY REFERENCES card(id) ON DELETE CASCADE,
    text       TEXT    NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

/// How many migrations this build knows.
pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

/// Bring `conn` up to [`CURRENT_VERSION`], or refuse a database from a newer build without
/// touching it.
pub fn migrate(conn: &mut Connection) -> Result<SchemaFacts, StoreError> {
    let recorded: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let version =
        usize::try_from(recorded).map_err(|_| StoreError::Corrupt { version: recorded })?;
    if version > MIGRATIONS.len() {
        return Err(StoreError::TooNew {
            found: recorded,
            supported: CURRENT_VERSION,
        });
    }
    let mut applied = 0;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        if index + 1 == NAMES {
            backfill_names(&tx)?;
        }
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
        applied += 1;
    }
    Ok(SchemaFacts {
        found: recorded,
        written: recorded + applied as i64,
        applied,
    })
}

/// Give every card that predates names the name its first side implies.
///
/// Runs inside V9's transaction, so a library either gains every name or none. The names are made
/// unique per deck with the same rule a save uses, so the backfill cannot leave two cards in one
/// deck answering to one name.
fn backfill_names(conn: &Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare("SELECT id, deck_id, kind, markdown FROM card ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let cards = rows.collect::<Result<Vec<_>, _>>()?;
    let mut taken: HashMap<i64, Vec<String>> = HashMap::new();
    for (id, deck, kind, markdown) in cards {
        // A diagram asks from its picture; its caption is not the card's text and never becomes
        // its name.
        let name = if kind == "diagram" {
            String::new()
        } else {
            let first = crate::sides::split(&markdown)
                .first()
                .copied()
                .unwrap_or("");
            crate::name::for_card(first, None, &crate::template::Fields::new(&[]))
        };
        let names = taken.entry(deck).or_default();
        let others = names.iter().map(String::as_str).collect::<Vec<_>>();
        let name = crate::name::unique(&name, &others);
        if !name.is_empty() {
            conn.execute("UPDATE card SET name = ?2 WHERE id = ?1", params![id, name])?;
            names.push(name);
        }
    }
    Ok(())
}

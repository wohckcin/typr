//! The schema, and the only code that changes it.

use rusqlite::Connection;

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
pub const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5, V6];

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

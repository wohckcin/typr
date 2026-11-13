//! The schema, and the only code that changes it.

use rusqlite::Connection;

use super::StoreError;

/// One SQL script per version, applied in order. `PRAGMA user_version` records how many ran.
pub const MIGRATIONS: &[&str] = &[V1];

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
pub fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > CURRENT_VERSION {
        return Err(StoreError::TooNew { found: version, supported: CURRENT_VERSION });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}


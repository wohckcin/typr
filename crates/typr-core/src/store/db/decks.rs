//! Deck mutations, hierarchy traversal, and deck-level template inheritance.

use rusqlite::{OptionalExtension, params};

use crate::command::{Command, Outcome};
use crate::edits;
use crate::error::StoreError;
use crate::model::Id;

use super::core::Db;

impl Db {
    pub(super) fn dispatch_deck(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::CreateDeck { parent, name } => {
                let name = edits::deck_name(name).ok_or(StoreError::BlankName { what: "deck" })?;
                self.named_write(
                    "INSERT INTO deck (parent_id, name) VALUES (?1, ?2)",
                    params![parent, name],
                    "deck under that parent",
                    name,
                )?;
                Outcome::Deck(self.conn.last_insert_rowid())
            }
            Command::RenameDeck { id, name } => {
                let name = edits::deck_name(name).ok_or(StoreError::BlankName { what: "deck" })?;
                let changed = self.named_write(
                    "UPDATE deck SET name = ?2 WHERE id = ?1",
                    params![id, name],
                    "deck under that parent",
                    name,
                )?;
                // `<<Deck>>` is the card's own deck name, so every card under this one says
                // something different now.
                for card in self.subtree_cards(*id)? {
                    self.rerender(card, now_ms)?;
                }
                Outcome::Changed(changed)
            }
            Command::ArchiveDeck { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET archived = ?2 WHERE id = ?1",
                    params![id, i64::from(*archived)],
                )?;
                Outcome::Changed(changed)
            }
            Command::MoveDeck { id, parent } => self.move_deck(*id, *parent, now_ms)?,
            Command::SetDeckTemplate { deck, template } => {
                self.set_deck_template(*deck, template.as_deref(), now_ms)?
            }
            Command::SetDeckReverse { deck, reverse } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET reverse = ?2 WHERE id = ?1",
                    params![deck, reverse.map(i64::from)],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "deck",
                        id: *deck,
                    });
                }
                Outcome::Changed(changed)
            }
            Command::SetDeckView { deck, view } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET view = ?2 WHERE id = ?1",
                    params![deck, view.as_str()],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "deck",
                        id: *deck,
                    });
                }
                Outcome::Changed(changed)
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    /// Give a deck a template its cards inherit, then re-render every card under it.
    ///
    /// One command, one transaction: a deck template change rewrites every card in that deck, and a
    /// failure halfway would otherwise leave some rendered against the new template and some
    /// against the old.
    fn set_deck_template(
        &self,
        deck: Id,
        template: Option<&str>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "UPDATE deck SET template = ?2 WHERE id = ?1",
            params![deck, template],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "deck",
                id: deck,
            });
        }
        let cards = self.subtree_cards(deck)?;
        for card in &cards {
            // A card's own template still wins: `rerender` reads the template in force, not this
            // deck's.
            self.rerender(*card, now_ms)?;
        }
        Ok(Outcome::Changed(cards.len()))
    }

    /// Every card in a deck and the decks under it.
    pub(super) fn subtree_cards(&self, deck: Id) -> Result<Vec<Id>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE under(id) AS (
                 SELECT ?1
                 UNION ALL
                 SELECT d.id FROM deck d JOIN under u ON d.parent_id = u.id
             )
             SELECT c.id FROM card c JOIN under u ON u.id = c.deck_id ORDER BY c.id",
        )?;
        let rows = stmt.query_map(params![deck], |row| row.get::<_, Id>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Move a deck under another, refusing a move that would put it inside its own subtree.
    pub fn move_deck(
        &self,
        id: Id,
        parent: Option<Id>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        if let Some(parent) = parent {
            let mut cursor = Some(parent);
            while let Some(deck) = cursor {
                if deck == id {
                    return Err(StoreError::Cycle { deck: id });
                }
                cursor = self
                    .conn
                    .query_row(
                        "SELECT parent_id FROM deck WHERE id = ?1",
                        params![deck],
                        |row| row.get::<_, Option<Id>>(0),
                    )
                    .optional()?
                    .flatten();
            }
        }
        let changed = self.conn.execute(
            "UPDATE deck SET parent_id = ?2 WHERE id = ?1",
            params![id, parent],
        )?;
        if changed > 0 {
            // Every card under the moved deck may now inherit a different template, or none.
            for card in self.subtree_cards(id)? {
                self.rerender(card, now_ms)?;
            }
        }
        Ok(Outcome::Changed(changed))
    }
}

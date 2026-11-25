//! Saved views, view filtering, and import/export command dispatch.

use rusqlite::params;

use crate::command::{Command, Outcome};
use crate::error::StoreError;
use crate::model::{CardSummary, ViewSpec};
use crate::transfer::{self, mochi};

use super::core::{Db, encode_tags, mode_text};

impl Db {
    pub(super) fn dispatch_view(
        &self,
        command: &Command,
        _now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::CardsForView { spec } => Outcome::Cards(self.cards_for_view(spec)?),
            Command::CreateView { spec, name } => {
                let name = name.trim();
                let changed = self.named_write(
                    "INSERT INTO view (name, deck, query, tags, mode, pinned_only, sort_key, sort_dir)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        name,
                        spec.deck,
                        spec.query,
                        encode_tags(&spec.tags),
                        mode_text(spec.mode),
                        i64::from(spec.pinned_only),
                        spec.sort.as_str(),
                        spec.dir.as_str(),
                    ],
                    "view",
                    name,
                )?;
                debug_assert_eq!(changed, 1, "one row is one view");
                Outcome::View(self.conn.last_insert_rowid())
            }
            Command::RenameView { id, name } => {
                let name = name.trim();
                let changed = self.named_write(
                    "UPDATE view SET name = ?2 WHERE id = ?1",
                    params![id, name],
                    "view",
                    name,
                )?;
                Outcome::Changed(changed)
            }
            Command::DeleteView { id } => Outcome::Changed(
                self.conn
                    .execute("DELETE FROM view WHERE id = ?1", params![id])?,
            ),
            Command::UpdateView { id, spec } => Outcome::Changed(self.conn.execute(
                "UPDATE view SET deck = ?2, query = ?3, tags = ?4, mode = ?5, pinned_only = ?6,
                                sort_key = ?7, sort_dir = ?8
                 WHERE id = ?1",
                params![
                    id,
                    spec.deck,
                    spec.query,
                    encode_tags(&spec.tags),
                    mode_text(spec.mode),
                    i64::from(spec.pinned_only),
                    spec.sort.as_str(),
                    spec.dir.as_str(),
                ],
            )?),
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    pub(super) fn dispatch_transfer(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::ImportArchive {
                archive,
                attachments,
            } => Outcome::Imported(mochi::import(
                self,
                archive,
                attachments,
                now_ms,
                || Ok(()),
            )?),
            // Export normally never reaches here: `apply` runs it before opening a transaction.
            // The arm keeps the match total and gives a clear failure if it is ever staged.
            Command::ExportLibrary { target } => {
                transfer::export_library(&self.path, target)?;
                Outcome::Exported
            }
            Command::ImportCsv { deck, source } => {
                Outcome::Imported(transfer::import_csv(self, *deck, source, now_ms)?)
            }
            Command::ExportCsv { deck, target } => {
                transfer::export_csv(self, *deck, target)?;
                Outcome::Exported
            }
            Command::ImportMarkdown { deck, source } => {
                Outcome::Imported(transfer::import_markdown(self, *deck, source, now_ms)?)
            }
            Command::ExportMarkdown { deck, target } => {
                transfer::export_markdown(self, *deck, target)?;
                Outcome::Exported
            }
            _ => return Ok(None),
        };
        Ok(Some(outcome))
    }

    /// The cards a filter selects, for a cram run: the library's list, minus what cannot be asked.
    ///
    /// The filter is the snapshot's own, so the drill and the list it was started from can never
    /// disagree about what is selected. Only the schedule is left out: a cram run has none, which
    /// is the whole point of it.
    pub fn cards_for_view(&self, spec: &ViewSpec) -> Result<Vec<CardSummary>, StoreError> {
        let snapshot = self.snapshot()?;
        Ok(snapshot
            .cards_matching(spec)
            .into_iter()
            .filter(|card| card.prompts > 0)
            .cloned()
            .collect())
    }
}

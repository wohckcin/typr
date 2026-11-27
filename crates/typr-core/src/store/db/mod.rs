//! The synchronous half of the store: one `Connection`, one command at a time.
//!
//! Every method here runs on the store task's thread and is exercised by tests directly, which is
//! what keeps the threaded part (`task`) down to plumbing.

mod cards;
mod core;
mod decks;
mod review;
mod snapshot;
mod views;

use crate::command::{Command, Domain, Outcome};
use crate::error::StoreError;
use crate::model::{Grade, Id, Snapshot};
use crate::transfer::{self, TransferTarget, mochi};

pub use self::core::Db;
#[cfg(test)]
pub(crate) use self::core::Record;

impl Db {
    /// Apply one command and report what it did. `now_ms` is the caller's clock, never this
    /// module's.
    ///
    /// The whole command is one transaction: a command that fails halfway leaves nothing behind,
    /// and a command that writes several rows pays one fsync.
    pub fn apply(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        // `VACUUM INTO` refuses to run inside a transaction, so the whole-library export is the
        // one command that does its work before the transaction rather than in it.
        if let Command::ExportLibrary { target } = command {
            transfer::export_library(&self.path, target)?;
            return Ok(Outcome::Exported);
        }
        let tx = self.conn.unchecked_transaction()?;
        // The import copies pictures before the rows commit, so it owns the commit to undo them.
        if let Command::ImportArchive {
            archive,
            attachments,
        } = command
        {
            let commit = move || tx.commit().map_err(StoreError::from);
            return mochi::import(self, archive, attachments, now_ms, commit)
                .map(Outcome::Imported);
        }
        let outcome = self.dispatch(command, now_ms)?;
        tx.commit()?;
        Ok(outcome)
    }

    /// Run one command inside the transaction the caller already holds. The import uses this so a
    /// forty-card archive pays one commit, and a failure halfway leaves no deck behind.
    pub(crate) fn staged(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        self.dispatch(command, now_ms)
    }

    /// The body of one command, run inside the transaction [`Db::apply`] opens.
    fn dispatch(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        let domain = command.domain();
        let handled = match domain {
            Domain::Deck => self.dispatch_deck(command, now_ms)?,
            Domain::Diagram => self.dispatch_diagram(command, now_ms)?,
            Domain::Card => self.dispatch_card(command, now_ms)?,
            Domain::CardState => self.dispatch_card_state(command, now_ms)?,
            Domain::Review => self.dispatch_review(command, now_ms)?,
            Domain::View => self.dispatch_view(command, now_ms)?,
            Domain::Transfer => self.dispatch_transfer(command, now_ms)?,
        };
        Ok(handled.unwrap_or_else(|| unreachable!("{} is routed to {domain:?}", command.name())))
    }
}

impl TransferTarget for Db {
    fn snapshot(&self) -> Result<Snapshot, StoreError> {
        Db::snapshot(self)
    }

    fn staged(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        Db::staged(self, command, now_ms)
    }

    fn record_review(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
        at_ms: i64,
    ) -> Result<Outcome, StoreError> {
        Db::record_review(self, card, key, grade, duration_ms, at_ms)
    }
}

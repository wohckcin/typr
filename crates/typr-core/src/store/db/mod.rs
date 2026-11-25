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

use crate::command::{Command, Outcome};
use crate::error::StoreError;
use crate::model::{Grade, Id, Snapshot};
use crate::transfer::{self, TransferTarget, mochi};

pub use self::core::Db;
#[cfg(test)]
pub(crate) use self::core::Record;

type DomainDispatcher = fn(&Db, &Command, i64) -> Result<Option<Outcome>, StoreError>;

const DISPATCHERS: &[DomainDispatcher] = &[
    Db::dispatch_deck,
    Db::dispatch_diagram,
    Db::dispatch_card,
    Db::dispatch_card_state,
    Db::dispatch_review,
    Db::dispatch_view,
    Db::dispatch_transfer,
];

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
        for handler in DISPATCHERS {
            if let Some(outcome) = handler(self, command, now_ms)? {
                return Ok(outcome);
            }
        }
        unreachable!("every Command variant is handled by a domain dispatcher")
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

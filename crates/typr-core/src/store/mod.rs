//! The store: one SQLite file, one writer thread, and `Store` as the only door.
//!
//! Views never touch a `Connection`: they submit a command and read the snapshot the store task
//! publishes. `db` holds the SQL and runs synchronously, so tests exercise it without a thread;
//! `task` is the single worker that owns the file.

mod db;
mod schema;
mod settings;
mod task;
mod view;

pub use crate::command::{Command, Event, Outcome, Store};
pub use crate::error::StoreError;
pub use crate::model::{
    CardKind, CardMode, CardRef, CardSummary, Deck, DuePrompt, Folded, Grade, Group, Id,
    MAX_RETENTION, MIN_RETENTION, MaskRect, MaskRow, NewPrompt, Phase, ReviewRow, SavedView,
    SchedulerKind, SettingKey, SettingValue, Settings, Snapshot, Sort, SortDir, Stats, TagBranch,
    Theme, ViewSpec, folded,
};
pub use db::Db;
pub use schema::{CURRENT_VERSION, SchemaFacts};
pub use settings::{MAX_NEW_PER_DAY, check_new_card_cap};
pub use task::{Handle, UNPROMPTED, Waker, WorkerStatus};
pub use view::shuffled;

#[cfg(test)]
mod tests;

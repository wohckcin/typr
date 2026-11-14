//! Where the library, its attachments and the app's folders live.
//!
//! The backend takes a path and writes what it is given; deciding which directory that is belongs to
//! the app, which is the only layer that knows about desktop conventions.

use std::path::PathBuf;

use rune::dirs::AppDirs;
use typr_core::store::{Db, StoreError};

/// The app id used for both Rune's folders and this app's file.
pub const APP_ID: &str = "typr";

/// The library file inside a data folder.
pub fn database_path(dirs: &AppDirs) -> PathBuf {
    dirs.data().join("typr.sqlite3")
}

/// Where a card's attached images are copied.
pub fn attachments_root(dirs: &AppDirs) -> PathBuf {
    dirs.data().join("attachments")
}

/// The logs folder of the root this process uses, created if needed.
pub fn logs_root_from_environment() -> Result<PathBuf, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    let logs = dirs.logs().to_path_buf();
    std::fs::create_dir_all(&logs)?;
    Ok(logs)
}

/// The attachments folder for the process environment.
pub fn attachments_root_from_environment() -> Result<PathBuf, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    Ok(attachments_root(&dirs))
}

/// The library file the process environment names, which is what a failure message needs.
pub fn database_path_from_environment() -> Result<PathBuf, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    Ok(database_path(&dirs))
}

/// Open the library from the process environment, creating its folder if needed.
///
/// `RUNE_DIRS_ROOT`, when set, moves every folder (see [`rune::dirs`]), which is how a test or an
/// isolated run keeps out of the real one.
pub fn open_from_environment() -> Result<Db, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    open_in(&dirs)
}

/// Open the library inside the given folders.
pub fn open_in(dirs: &AppDirs) -> Result<Db, StoreError> {
    std::fs::create_dir_all(dirs.data())?;
    std::fs::create_dir_all(attachments_root(dirs))?;
    Db::open(&database_path(dirs))
}

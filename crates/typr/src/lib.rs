//! `typr` — a local-first flashcard app on Native Shell.
//!
//! The screens and the `ui` layer draw with `nsh-core`/`nsh-base`; the scheduling, the library and
//! the session rules live in `typr-core`, which knows no toolkit.

use nsh_core::prelude::*;

/// The crate's UI set, which `nsh.json` names (`"rust": "crates/typr"`). The app's root is the Rust
/// component `shell::App`, so nothing is registered for TSX to import; the function is what makes
/// the crate a UI set at all.
#[native_module("typr/native")]
fn native(_registry: &mut Registry) {}

pub mod paths;
pub mod screens;
pub mod shell;
pub mod ui;

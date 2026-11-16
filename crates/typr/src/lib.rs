//! `typr` — a local-first flashcard app on Native Shell.
//!
//! The screens and the `ui` layer draw with `nsh-core`/`nsh-base`; the scheduling, the library and
//! the session rules live in `typr-core`, which knows no toolkit.

pub mod paths;
pub mod screens;
pub mod shell;
pub mod ui;

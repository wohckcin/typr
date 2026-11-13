//! Which screen the app is showing, and the two queues a session can walk.

/// A queue a session walks: cards never reviewed, or prompts that have fallen due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Queue {
    New,
    Due,
}

/// The screen the app is on. Each screen owns its own transitions: the library answers the
/// keyboard, the editor asks before it closes, and a session's order lives in `session::Session`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Library,
    Editor,
    Session,
    Summary,
}

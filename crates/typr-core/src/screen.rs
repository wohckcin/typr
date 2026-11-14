//! Which queues a session can walk.

/// A queue a session walks: cards never reviewed, or prompts that have fallen due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Queue {
    New,
    Due,
}

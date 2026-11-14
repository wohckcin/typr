# Spec Delta — project-structure

## MODIFIED Requirements

### Requirement: A module's tests live where the module is readable

The system SHALL keep the tests of a module close enough to it that a reader meets one subject at a time:
a module whose tests run longer than its code SHALL keep them in a `tests.rs` beside it, and a module whose
tests fit inside it SHALL carry them inline behind `#[cfg(test)] mod tests`. Where the tests sit beside the
code in a folder, the module's own root SHALL sit in that folder too, as `foo/mod.rs`, so a directory is
never created only to hold tests.

#### Scenario: A large module keeps its tests beside it

- **WHEN** a module carries more than a few hundred lines of test
- **THEN** those tests live in `foo/tests.rs` and the module's root is `foo/mod.rs`

#### Scenario: A small module carries its own tests

- **WHEN** a module is shorter than its test file would be
- **THEN** the tests sit inline in that module, and no folder exists for it

#### Scenario: A binary's tests name their subject

- **WHEN** the application's tests grow past one subject
- **THEN** they are split into modules named for what they check, and no crate-root test module holds them all

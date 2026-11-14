# Spec Delta — project-structure

## ADDED Requirements

### Requirement: A module's tests live where the module is readable

The system SHALL keep the tests of a module close enough to it that a reader meets one subject at a time:
a module whose tests run longer than its code SHALL keep them in a sibling `tests.rs` next to it, and a
module whose tests fit inside it SHALL carry them inline behind `#[cfg(test)] mod tests`. A crate SHALL NOT
declare tests from its root that belong to no module.

#### Scenario: A large module keeps its tests beside it

- **WHEN** a module carries more than a few hundred lines of test
- **THEN** those tests live in a sibling `tests.rs` declared with `#[cfg(test)] mod tests;`

#### Scenario: A small module carries its own tests

- **WHEN** a module is shorter than its test file would be
- **THEN** the tests sit inline in that module, and no separate test file exists for it

#### Scenario: A binary's tests name their subject

- **WHEN** the application's tests grow past one subject
- **THEN** they are split into modules named for what they check, and no crate-root test module holds them all

# project-structure Specification

## Purpose
The crate boundaries of this project: what the backend library may depend on, where it gets the
facts it needs (its storage location and the study-day offset), and where the desktop toolkit is
allowed to appear. These are build-time and API-shape properties rather than screen behaviour, and
they exist so the library outlives any one front end.

## Requirements

### Requirement: The backend builds without the desktop toolkit

The backend library MUST compile and pass its tests with no desktop-toolkit crate in its dependency
graph, and in an environment with no display server.

#### Scenario: The dependency graph is clean

- **WHEN** the backend crate's normal dependencies are listed
- **THEN** no NSH crate (`nsh-*`) and no Rune crate appears in the output

#### Scenario: The backend tests run headless

- **WHEN** the backend crate's tests are run on a machine with no `DISPLAY`, no `WAYLAND_DISPLAY`
  and no GTK or GPU libraries
- **THEN** they pass, because the backend links no windowing or GPU code

### Requirement: The backend takes its storage location from the caller

Opening a library MUST be driven by a path the caller supplies. No function in the backend may
resolve a platform application-data directory, a home directory or an environment variable to find
one.

#### Scenario: A library is opened where the caller says

- **WHEN** a caller opens a library in a temporary directory
- **THEN** the database and its attachment folder appear under that directory and nothing is written
  anywhere else

#### Scenario: No ambient directory lookup

- **WHEN** the backend's public API is inspected for functions that decide where a library lives
- **THEN** there are none: the location is a parameter, and any environment-variable override belongs
  to the front end that supplies it

### Requirement: The study day's offset comes from the caller

Every backend query that depends on which study day a moment falls in MUST accept the offset from
its caller. The backend MUST NOT read the machine's timezone itself.

#### Scenario: Two callers, two zones, one library

- **WHEN** the same library is asked for its due queue and its daily statistics with two different
  offsets
- **THEN** each answer reflects the offset it was given, and neither answer changes the stored
  schedule

#### Scenario: A frozen offset cannot leak in

- **WHEN** the backend is used from a process whose local timezone is unknown or unreadable
- **THEN** its behaviour depends only on the offsets passed with each call, so a caller that supplies
  a zone still gets that zone's days

### Requirement: The desktop crate is the only consumer of the toolkit

Toolkit and windowing dependencies MUST appear only in the desktop crate. The binary, its screens and
their screenshot tests remain a desktop concern.

#### Scenario: Toolkit crates appear once

- **WHEN** the whole workspace's dependency graph is inspected
- **THEN** the `nsh-*` crates are reached only through the desktop crate

#### Scenario: The app still draws after the move

- **WHEN** the desktop crate's screenshot tests run on a supported platform
- **THEN** every screen's screenshot is written and is not empty

### Requirement: A library written before the split opens unchanged

The split MUST NOT change the storage format or the schema version, so an existing library keeps
working.

#### Scenario: An existing library is used after the split

- **WHEN** a library file written by the previous build is opened by the split build
- **THEN** its decks, cards, prompts and review records are unchanged, and grading a card appends to
  the same history it would have before

### Requirement: The workspace builds and tests with one command

The whole workspace MUST build and test under one locked command, so a change that breaks one crate
cannot be merged while the other passes.

#### Scenario: One command covers both crates

- **WHEN** the workspace's tests are run with `--locked`
- **THEN** both the backend and the desktop crate's tests run, and a failure in either fails the
  command

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

### Requirement: One NSH source for the whole workspace

The workspace MUST resolve every `nsh-*` crate from one source at one version, declared once at the workspace root.

#### Scenario: A bump is one edit

- **WHEN** NSH is moved to a newer version
- **THEN** one workspace dependency table changes, and every `nsh-*` crate in `Cargo.lock` resolves to that version

#### Scenario: Mixed sources are refused

- **WHEN** the resolved dependency graph is inspected
- **THEN** every `nsh-*` package comes from the same source and version, and no Rune package is present

### Requirement: A public struct widens without breaking its callers

A public struct with public fields SHALL be `#[non_exhaustive]`, so a caller outside the crate cannot build
it with a struct literal and a later field is an addition. It SHALL offer a construction that sets only what
the caller means to set: `Default` plus field assignment, or a constructor. A struct that owns reactive
state MAY omit `Default` when its constructor is its only way to exist and the type documents why.

#### Scenario: A caller outside the crate still builds it

- **WHEN** a caller in another crate builds a public struct with public fields
- **THEN** it compiles through the type's `Default` and assignment or through its constructor, and no struct
  literal or functional update is needed

#### Scenario: A field is added

- **WHEN** a field is added to a public struct with public fields
- **THEN** no caller outside the crate needs a change

#### Scenario: Reactive state names its constructor

- **WHEN** a struct's fields are signals its caller's owner creates
- **THEN** it is built only through its constructor, and it documents why it carries no `Default`

### Requirement: A component's events carry one named object

Each event a component reports SHALL carry one object whose fields name the values it reports, and its
name SHALL say what the user did. A lone value or a data type SHALL NOT be the payload itself.

#### Scenario: Payloads name their values

- **WHEN** the editor reports that the card was saved
- **THEN** the handler receives an object whose field names the text

#### Scenario: Handlers read as acts

- **WHEN** a component's event names are listed
- **THEN** each names the act the user performed rather than the state it left behind

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
- **THEN** no Rune crate (the drawing toolkit, its component library, or their wgpu/winit/parley
  stack) appears in the output

#### Scenario: The backend tests run headless

- **WHEN** the backend crate's tests are run on a machine with no `DISPLAY`, no `WAYLAND_DISPLAY`
  and no Vulkan driver
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

### Requirement: One Rune revision for the whole workspace

The workspace MUST pin the drawing toolkit to a single revision, and the patch entries that revision
requires MUST live at the workspace root, since a member's patches never apply to its dependents.

#### Scenario: A bump is one edit

- **WHEN** the toolkit is moved to a newer revision
- **THEN** the revision and its patch entries are changed in one file, and every member resolves the
  same revision

#### Scenario: A member cannot drift

- **WHEN** the resolved dependency graph of either member is inspected
- **THEN** both name the same toolkit revision, and no member carries a patch entry of its own

### Requirement: The desktop crate is the only consumer of the toolkit

Toolkit and windowing dependencies MUST appear only in the desktop crate. The binary, its scenes and
their snapshots remain a desktop concern.

#### Scenario: Toolkit crates appear once

- **WHEN** the whole workspace's dependency graph is inspected
- **THEN** the toolkit crates are reached only through the desktop crate

#### Scenario: The app still draws after the move

- **WHEN** the desktop crate renders each scene to a snapshot file
- **THEN** every snapshot is written and is not empty

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


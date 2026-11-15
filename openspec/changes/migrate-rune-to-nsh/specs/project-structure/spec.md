## MODIFIED Requirements

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

### Requirement: The desktop crate is the only consumer of the toolkit

Toolkit and windowing dependencies MUST appear only in the desktop crate. The binary, its screens and
their screenshot tests remain a desktop concern.

#### Scenario: Toolkit crates appear once

- **WHEN** the whole workspace's dependency graph is inspected
- **THEN** the `nsh-*` crates are reached only through the desktop crate

#### Scenario: The app still draws after the move

- **WHEN** the desktop crate's screenshot tests run on a supported platform
- **THEN** every screen's screenshot is written and is not empty

## ADDED Requirements

### Requirement: One NSH source for the whole workspace

The workspace MUST resolve every `nsh-*` crate from one source at one version, declared once at the workspace root.

#### Scenario: A bump is one edit

- **WHEN** NSH is moved to a newer version
- **THEN** one workspace dependency table changes, and every `nsh-*` crate in `Cargo.lock` resolves to that version

#### Scenario: Mixed sources are refused

- **WHEN** the resolved dependency graph is inspected
- **THEN** every `nsh-*` package comes from the same source and version, and no Rune package is present

## REMOVED Requirements

### Requirement: One Rune revision for the whole workspace

**Reason**: Rune is no longer a dependency, and NSH crates need no workspace patch entries.
**Migration**: Replaced by "One NSH source for the whole workspace".

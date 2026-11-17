## ADDED Requirements

### Requirement: The app opens on the library from its run path

The app SHALL open on the library when started with `nsh run`, the path the README documents, on a machine
with no `TYPR_DATA_DIR` set, and SHALL NOT depend on a platform host to establish engine state before the app
reads it.

#### Scenario: The documented run path

- **WHEN** `nsh run` starts the app on a machine with no `TYPR_DATA_DIR`
- **THEN** the window shows the sidebar and the library grid
- **AND** no panic reaches the process's stderr

#### Scenario: A folder that cannot be used

- **WHEN** the data folder is missing or cannot be written
- **THEN** the app still opens and states which folder failed
- **AND** the process exits non-zero only after the message has been shown

### Requirement: A fact the engine owns is read where the engine runs

The app SHALL resolve the platform's data folder from inside the engine's runtime, and SHALL NOT read an
ambient engine value before the engine has mounted.

#### Scenario: Resolving the folder

- **WHEN** the app's first mount asks for its data folder without `TYPR_DATA_DIR`
- **THEN** the value comes from the engine's ambient data directory
- **AND** the same call made before the engine mounts fails in a test rather than at the user's first launch

### Requirement: A control's props live as long as the control

A component SHALL create every prop it hands to a `nsh_base` control, and SHALL NOT pass a prop whose owner is
a scope that can be disposed while the control exists.

#### Scenario: A second render

- **WHEN** the app root renders, its selection changes, and it renders again
- **THEN** no control reads a disposed prop
- **AND** the screen shows the second state

#### Scenario: A screen re-rendered after a rebuild

- **WHEN** a screen that holds a button or a toggle is rebuilt by the shell
- **THEN** the control's label and disabled state are the new ones
- **AND** no panic is logged on the engine thread

# app-startup Specification

## Purpose

How the app comes up: the run path it opens on, where the data folder is read, and how long a control's
props live.

## Requirements

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

### Requirement: A library with no deck is given one

The app SHALL give a library that holds no deck one deck when it opens, so the library's empty screen has
somewhere to put the card its action makes, and SHALL NOT add another deck to a library that already holds one.

#### Scenario: The library does not exist yet

- **WHEN** the app opens on a data folder that holds no database
- **THEN** the sidebar lists one deck
- **AND** the grid states the deck holds no cards and offers the action that makes one

#### Scenario: The empty deck's action

- **WHEN** the deck holds no cards and the user presses the action the empty state offers
- **THEN** a card is created in that deck and its editor opens
- **AND** the card is in the library the next time it is opened

#### Scenario: A library that already holds a deck

- **WHEN** the app opens on a library that holds a deck, archived ones included
- **THEN** no deck is written
- **AND** the sidebar lists the decks the library already had

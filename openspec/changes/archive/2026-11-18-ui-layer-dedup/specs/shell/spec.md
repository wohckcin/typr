## MODIFIED Requirements

### Requirement: A screen exists once

The app SHALL draw each screen from one function, and SHALL NOT draw a second copy of a screen for a state of the
library — an open and a closed library differ in what a command does, not in which screen is drawn. A branch that
falls back to a screen SHALL draw it with that screen's function.

#### Scenario: The library with a store and without one

- **WHEN** the app opens with a library and again with none
- **THEN** both draw the same library screen, built by the same function
- **AND** the rows, the search field and the grid are the same elements in both

#### Scenario: A command that needs the store

- **WHEN** the library is closed and a store command is run from its key
- **THEN** the screen says the library is closed
- **AND** the key that runs it is still bound

#### Scenario: A card gone under the editor

- **WHEN** the card open in the editor leaves the snapshot
- **THEN** the library shown until the route moves back is drawn by the library screen's function

## ADDED Requirements

### Requirement: The store has one writer

Every command the shell sends to the store SHALL go through one writer, which times it, records it when slow,
and flags a stoppage when the store refuses it.

#### Scenario: A slow grade is recorded

- **WHEN** a grade takes longer than the slow-command threshold
- **THEN** the diagnostics screen lists it, as it lists a slow save

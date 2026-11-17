## ADDED Requirements

### Requirement: A screen exists once

The app SHALL draw each screen from one function, and SHALL NOT draw a second copy of a screen for a state of the
library — an open and a closed library differ in what a command does, not in which screen is drawn.

#### Scenario: The library with a store and without one

- **WHEN** the app opens with a library and again with none
- **THEN** both draw the same library screen, built by the same function
- **AND** the rows, the search field and the grid are the same elements in both

#### Scenario: A command that needs the store

- **WHEN** the library is closed and a store command is run from its key
- **THEN** the screen says the library is closed
- **AND** the key that runs it is still bound

### Requirement: A screen the tests render is the screen the app renders

A test that asserts how a screen looks or behaves SHALL render the function the app renders for that screen.

#### Scenario: The appearance pass

- **WHEN** the light and dark appearance pass renders the library
- **THEN** it renders the function the shell draws the library with
- **AND** a second library drawing fails the pass rather than going unnoticed

#### Scenario: A state the app cannot produce

- **WHEN** a shell test renders the library
- **THEN** its arguments are a combination `App()` can pass — an open library with a store, or none with an empty
  snapshot
- **AND** a fixture with cards is rendered through the shell that a store feeds

#### Scenario: Keyboard movement

- **WHEN** the library holds cards and the arrow keys are pressed
- **THEN** the selection moves through the grid's cards
- **AND** Home and End reach the first and the last

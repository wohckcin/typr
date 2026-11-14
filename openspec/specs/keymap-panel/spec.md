# keymap-panel Specification

## Purpose

How typr decides which key does what, and how it tells the user: one table declares every binding once, so a chord cannot mean two things on a screen, the shortcut panel can only list what is really bound, and a control can show the key that runs it.

## Requirements

### Requirement: One table declares every binding

The system SHALL declare each command's key chord and surface in one table, SHALL dispatch keys through that
table, and SHALL draw keycap hints from it, so that no chord is handled anywhere else.

#### Scenario: A chord in the table works everywhere it applies

- **WHEN** the table binds a chord on the library surface and the library is open
- **THEN** pressing the chord runs that command

#### Scenario: A chord absent from the table does nothing

- **WHEN** a key that the table does not bind is pressed on a surface
- **THEN** nothing happens and no error is shown


### Requirement: A chord cannot mean two things on one surface

The system SHALL reject, in a test, any table that binds one chord to two commands on the same surface, and
SHALL allow the same chord to differ between surfaces.

#### Scenario: A duplicate is refused

- **WHEN** a chord is bound twice on the library surface
- **THEN** the test fails and names both commands

#### Scenario: The same chord may differ per surface

- **WHEN** one chord is bound to reveal in the session and to something else in the editor
- **THEN** the table is accepted and each surface dispatches its own command


### Requirement: The shortcut panel lists the bindings it can run

The system SHALL open a panel on the shortcut key that lists every binding for the current surface and the
surfaces reachable from it, grouped by surface, SHALL offer a text filter over the labels, and SHALL NOT list a
binding the current surface cannot run.

#### Scenario: The panel opens and closes on the same key

- **WHEN** the shortcut key is pressed and then pressed again
- **THEN** the panel opens and then closes

#### Scenario: Filtering narrows the list

- **WHEN** the user types a word into the panel's filter
- **THEN** only bindings whose label matches are listed


### Requirement: An action shows its keycap

The system SHALL draw the binding's keycap beside any action that has one, in the session's binding row and on
the controls that trigger it, so the keyboard model is visible without opening the panel.

#### Scenario: The reveal action carries its key

- **WHEN** the session screen is drawn before the first reveal
- **THEN** the reveal action shows the key that reveals


### Requirement: The command palette runs the commands of the current surface

The system SHALL open a palette on the palette chord, SHALL list the commands valid on the current surface,
SHALL narrow that list as the user types, and SHALL run the chosen command on Enter.

#### Scenario: Typing narrows and Enter runs

- **WHEN** the palette is open and the user types enough of a command's name to select it and presses Enter
- **THEN** that command runs and the palette closes

#### Scenario: An unavailable command is not offered

- **WHEN** the palette is opened on the library surface
- **THEN** session-only commands are not listed

# Preferences Specification

## Purpose

The library's own settings are reachable from inside the app, and a change takes effect without a restart or an edit to the file by hand.

## Requirements

### Requirement: Settings live in the library

The system SHALL store every user setting in the library's own database so that a setting travels with a
library, SHALL apply a stored setting at startup, and SHALL fall back to a documented default for a
setting that is missing.

#### Scenario: A copied library keeps its settings

- **WHEN** a library is copied to another machine and opened
- **THEN** every setting is the value it was set to, not the new machine's default

#### Scenario: A missing setting uses its default

- **WHEN** the settings table holds no value for a key
- **THEN** the app runs with that key's documented default and does not fail to open

### Requirement: A setting is validated before it is stored

The system SHALL refuse a value outside its allowed range or of the wrong shape, SHALL keep the previous
value when it refuses, SHALL say which setting and which value were refused, and SHALL report a stored
value that cannot be read rather than silently replacing it.

#### Scenario: An out-of-range number is refused

- **WHEN** a new-card cap of 100000 is entered
- **THEN** the value is refused, the old one stays, and the screen names the key and the limit

#### Scenario: A corrupt stored value is reported

- **WHEN** the settings table holds "abc" for a numeric key
- **THEN** the app reports that key and value instead of running with a default

### Requirement: Settings are reachable from the app

The system SHALL provide a settings screen reachable from the library and from a session, SHALL list its
settings as rows in one card in the order Review, Appearance, Library, and SHALL change the running app's
behaviour as soon as a setting is accepted, without a restart. The Appearance rows SHALL offer a control for every
appearance setting the library stores, and the appearance the app draws SHALL be the one that setting holds.

#### Scenario: The screen is reachable from a session

- **WHEN** the user presses the settings binding during a review
- **THEN** the settings screen opens, and leaving it returns to the review at the same card

#### Scenario: A change takes effect at once

- **WHEN** a setting is accepted
- **THEN** the next frame follows it, and a restart keeps it

#### Scenario: Changing the appearance

- **WHEN** the reader chooses Light in the Appearance rows
- **THEN** the frame after the change draws in the light appearance
- **AND** the value is in the library's settings table before that frame is drawn

#### Scenario: The stored appearance at launch

- **WHEN** the app opens a library whose stored appearance is Dark, on a desktop set to Light
- **THEN** the first frame draws in the dark appearance
- **AND** no frame draws in the other one first

#### Scenario: A stored setting with no control

- **WHEN** a setting the library stores is compared with the Appearance rows' controls
- **THEN** it has a control, or the setting is not stored

### Requirement: The screen reports where the library is

The system SHALL show the database path, its schema version, its size, the attachments folder and the log
path, and SHALL offer the import and export actions.

#### Scenario: The paths are shown and can be opened

- **WHEN** the settings screen is opened
- **THEN** each path is displayed in full and offers a control that reveals it in the file manager

#### Scenario: The schema version is the file's

- **WHEN** a library written by an older build is opened
- **THEN** the screen reports the version that file is at, and that it was migrated on open

### Requirement: A settings field carries a visible label

A setting that is edited through a text field SHALL name itself with visible text beside the field. The
field's placeholder SHALL NOT be its only name, and the label SHALL stay visible while the field holds a
value.

#### Scenario: The label survives a value

- **WHEN** the daily new-card limit holds a number
- **THEN** the text naming the setting is still visible beside the field

#### Scenario: The setting is named without its placeholder

- **WHEN** the settings screen is read with the field's placeholder hidden
- **THEN** the purpose of the field is still stated by the label

### Requirement: A setting is a label and its control in one row

Each setting SHALL be one row of a single card: its label at the leading edge and its control at the trailing
edge. Rows SHALL share one height and one hairline between each pair, drawn once by the card, and no control SHALL
stretch to the width of the window.

#### Scenario: Label and control share a row

- **WHEN** the settings screen is drawn at 1920 × 1080
- **THEN** each setting's label is at the card's leading edge and its control at the trailing edge, on one line

#### Scenario: A value that is read is a row too

- **WHEN** the Library rows are drawn
- **THEN** each path is the row's value, with its reveal control beside it, and the schema version and size are
  value rows with no control

### Requirement: A choice among a few values is a segmented control

A setting that takes one of a few named values (the theme, a flag) SHALL be drawn as a segmented control: every
value visible at once, the chosen one marked by a raised fill and exposed as selected, each segment a button named
by its value.

#### Scenario: Choosing a segment

- **WHEN** the reader presses `Light` in the theme control
- **THEN** that segment becomes the selected one and the others do not

#### Scenario: A flag is Off and On

- **WHEN** "Show review counts" is drawn
- **THEN** it offers the segments `Off` and `On`, and the one in force is selected

### Requirement: A setting can be found by name

The settings screen SHALL carry a header with the breadcrumb "Settings / General" and a field labelled "Find a
setting". Typing SHALL hide every row whose label does not contain the text, ignoring case, and SHALL say so when no
row is left.

#### Scenario: Typing narrows the rows

- **WHEN** the reader types `theme` in the field
- **THEN** only the Theme row is drawn

#### Scenario: Nothing matches

- **WHEN** the reader types text that no label contains
- **THEN** no row is drawn and the screen says no setting matches

#### Scenario: Clearing restores every row

- **WHEN** the field is emptied
- **THEN** every row is drawn again

### Requirement: Diagnostics is reachable from Settings

The settings screen SHALL offer a row that opens the diagnostics screen, because the sidebar no
longer has a Diagnostics row.

#### Scenario: Opening diagnostics from settings
- **WHEN** the user opens Settings and presses the Diagnostics row's control
- **THEN** the diagnostics screen is shown

# Spec — preferences

## ADDED Requirements

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

## ADDED Requirements

### Requirement: Settings are reachable from the app

The system SHALL provide a settings screen reachable from the library and from a session, SHALL group
its controls into Review, Appearance and Library, and SHALL change the running app's behaviour as soon as
a setting is accepted, without a restart.

#### Scenario: The screen is reachable from a session

- **WHEN** the user presses the settings binding during a review
- **THEN** the settings screen opens, and leaving it returns to the review at the same card

#### Scenario: A change takes effect at once

- **WHEN** the interface scale is changed
- **THEN** the next frame is drawn at the new scale, and a restart keeps it

### Requirement: The screen reports where the library is

The system SHALL show the database path, its schema version, its size, the attachments folder and the log
path, and SHALL offer the import and export actions.

#### Scenario: The paths are shown and can be opened

- **WHEN** the settings screen is opened
- **THEN** each path is displayed in full and offers a control that reveals it in the file manager

#### Scenario: The schema version is the file's

- **WHEN** a library written by an older build is opened
- **THEN** the screen reports the version that file is at, and that it was migrated on open

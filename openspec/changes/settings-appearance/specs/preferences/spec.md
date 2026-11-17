## MODIFIED Requirements

### Requirement: Settings are reachable from the app

The system SHALL provide a settings screen reachable from the library and from a session, SHALL group
its controls into Review, Appearance and Library, and SHALL change the running app's behaviour as soon as
a setting is accepted, without a restart. The Appearance group SHALL offer a control for every appearance
setting the library stores, and the appearance the app draws SHALL be the one that setting holds.

#### Scenario: The screen is reachable from a session

- **WHEN** the user presses the settings binding during a review
- **THEN** the settings screen opens, and leaving it returns to the review at the same card

#### Scenario: A change takes effect at once

- **WHEN** a setting is accepted
- **THEN** the next frame follows it, and a restart keeps it

#### Scenario: Changing the appearance

- **WHEN** the reader chooses Light in the Appearance group
- **THEN** the frame after the change draws in the light appearance
- **AND** the value is in the library's settings table before that frame is drawn

#### Scenario: The stored appearance at launch

- **WHEN** the app opens a library whose stored appearance is Dark, on a desktop set to Light
- **THEN** the first frame draws in the dark appearance
- **AND** no frame draws in the other one first

#### Scenario: A stored setting with no control

- **WHEN** a setting the library stores is compared with the Appearance group's controls
- **THEN** it has a control, or the setting is not stored

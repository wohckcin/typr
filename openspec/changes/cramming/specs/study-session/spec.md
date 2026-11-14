# Spec Delta — study-session

## ADDED Requirements

### Requirement: A cram session visits what the filter selects

The system SHALL start a study session from any filter the library can express, including a saved view,
SHALL visit the cards that filter selects regardless of their due dates, and SHALL NOT apply the
per-day new-card cap to them.

#### Scenario: Cramming a tag with nothing due

- **WHEN** every card tagged `exam` is scheduled for next week and the user starts a cram session on
  that tag
- **THEN** the session visits those cards now, and no other card

#### Scenario: The cap does not gate a cram session

- **WHEN** the new-card cap has already been reached today
- **THEN** a cram session still visits the cards its filter selects

#### Scenario: An empty filter starts nothing

- **WHEN** the filter selects no card
- **THEN** the session does not open and the screen says there is nothing to cram

### Requirement: A cram session writes no schedule

The system SHALL, in a cram session, record no review, change no phase, interval or due date, and SHALL
count each answer in the session's own summary. Leaving a cram session SHALL leave every prompt exactly
as it was.

#### Scenario: A remembered cram answer changes nothing

- **WHEN** a card whose due date is next week is answered remembered in a cram session
- **THEN** its prompt still holds the same phase, interval and due date, and holds no new record

#### Scenario: A forgotten cram answer changes nothing either

- **WHEN** the same card is answered forgot in a cram session
- **THEN** no record is written and the card returns later in the same session

### Requirement: A cram session repeats what was forgotten

The system SHALL return a card answered forgot in a cram session to the end of that session's queue,
SHALL finish only when every card has been answered remembered, and SHALL NOT stop for a fixed number
of passes.

#### Scenario: One card, three passes

- **WHEN** the only card in a cram session is answered forgot twice and remembered once
- **THEN** the session shows it three times and finishes after the third answer

#### Scenario: Finishing reports the cram run

- **WHEN** the session ends
- **THEN** the summary reports how many answers were given and that nothing was scheduled

### Requirement: A cram session can be kept as a view

The system SHALL let the user save a cram session's filter as a saved view from the summary, SHALL
store it with the session's filter unchanged, and SHALL NOT save one on its own.

#### Scenario: Saving the drill

- **WHEN** the user saves a cram session on the tag `exam` as "Exam drill"
- **THEN** a view with that name selects the same cards, and no view is created unless asked

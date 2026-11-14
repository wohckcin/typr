# Spec Delta — review-scheduling

## ADDED Requirements

### Requirement: The scheduler can be chosen

The system SHALL keep the fixed multipliers as the default scheduler, SHALL let FSRS be chosen instead,
SHALL apply a scheduler change to future answers only, and SHALL NOT alter a prompt's history when the
scheduler changes.

#### Scenario: Fixed multipliers are the default

- **WHEN** a library is created and nothing is changed
- **THEN** a remembered answer lengthens the interval by the fixed multiplier, as it does today

#### Scenario: Switching does not rewrite the past

- **WHEN** a library with a hundred reviewed prompts switches to FSRS
- **THEN** every record and interval is unchanged, and the next answer of each prompt is computed by
  FSRS

#### Scenario: Switching back is symmetric

- **WHEN** the library switches back to the fixed scheduler
- **THEN** further answers use the multipliers and the FSRS state is kept for a later switch back

### Requirement: FSRS computes intervals from memory state

The system SHALL, under FSRS, keep a stability and a difficulty per prompt, SHALL update them on each
grade, SHALL choose a due date from the target retention, and SHALL use the two binary grades as FSRS's
`Again` and `Good`.

#### Scenario: A forgotten card comes back sooner than a remembered one

- **WHEN** two prompts with the same history are graded, one forgot and one remembered, under FSRS
- **THEN** the forgotten prompt's due date is earlier and its stability is lower

#### Scenario: A prompt without memory state is bootstrapped

- **WHEN** FSRS grades a prompt that has an interval but no stability
- **THEN** a stability is derived from that interval, the grade is applied on top, and no record is
  rewritten

### Requirement: The target retention is settable

The system SHALL let the target retention be set between 70% and 97%, SHALL default it to 90%, SHALL
reject a value outside that range, and SHALL use it only for interval length.

#### Scenario: A higher retention shortens intervals

- **WHEN** one library is set to 70% and another to 95% and both grade an identical card remembered
- **THEN** the 95% library schedules it sooner

#### Scenario: An impossible retention is refused

- **WHEN** a retention of 150% is stored
- **THEN** the setting is refused and the previous value stays

### Requirement: The review history can be exported for tuning

The system SHALL produce the review history as a table of card, review time, grade and the schedule the
review produced, in a shape an external FSRS optimiser reads, and SHALL NOT run the optimiser itself.

#### Scenario: An exported history is complete

- **WHEN** a library with a hundred records is exported
- **THEN** the export holds one row per record with its grade and timestamp, and no records are lost

#### Scenario: Weights can be replaced

- **WHEN** a weights list produced elsewhere is stored
- **THEN** the next FSRS grade uses it, and a malformed list is refused without changing the old one

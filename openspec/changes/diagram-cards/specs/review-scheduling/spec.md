# Spec Delta — review-scheduling

## ADDED Requirements

### Requirement: A mask group is a prompt

The system SHALL give every mask group of a diagram card one prompt keyed `mask:<n>`, SHALL keep its
phase, interval, due date and history separate from every other prompt of that card, and SHALL grade
it with the same two binary grades.

#### Scenario: Two groups, two schedules

- **WHEN** a diagram card holds groups 1 and 2
- **THEN** it has prompts `mask:1` and `mask:2`, and grading one leaves the other's due date alone

#### Scenario: A diagram card without groups does not queue

- **WHEN** a diagram card holds an image and no rectangles
- **THEN** it has no prompt and never appears in a study queue

### Requirement: The studyable view admits diagram cards

The system SHALL treat a diagram card as studyable when it holds at least one mask group, in addition
to the existing rule for a card with two or more sides, and SHALL keep excluding cards that are
archived, trashed, or under an archived deck.

#### Scenario: A masked diagram queues

- **WHEN** a diagram card with one group sits in an ordinary deck
- **THEN** it appears in the new queue for its `mask:1` prompt

#### Scenario: A trashed diagram does not queue

- **WHEN** a diagram card with a group is moved to the trash
- **THEN** both of its queues stop listing it and its history remains

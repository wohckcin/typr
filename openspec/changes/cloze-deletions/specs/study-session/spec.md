# Spec Delta — study-session

## MODIFIED Requirements

### Requirement: Sides are revealed in order within one review

The system SHALL show one face of the prompt under review at a time, SHALL show the first face and
reveal each later face only on the user's request, in order, with the grade offered only after the last
face has been revealed. This SHALL hold in both queues. Revealing faces SHALL constitute one review,
not several. A face SHALL carry the text to show while hidden and the text to show once revealed: for a
card built from sides the two are the same, and for a hidden-text group they differ only in the masked
span.

#### Scenario: Three-sided card is one review

- **WHEN** the user studies a card with three sides
- **THEN** the sides are revealed in order and a single grade is recorded for the prompt

#### Scenario: Grading is unavailable before the last side

- **WHEN** the first side of a two-sided card is showing
- **THEN** the remembered and forgot actions are not available until the second side is revealed

#### Scenario: One hidden-text face unmasks

- **WHEN** the prompt `cloze:1` is reviewed
- **THEN** the sentence is shown with that group masked, one reveal unmasks it, and the grade is
  offered straight away

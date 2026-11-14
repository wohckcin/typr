# Spec Delta — study-session

## ADDED Requirements

### Requirement: A reverse review shows the last side first

The system SHALL, when reviewing the `rev` prompt of a card, show the card's last side as the face to
be answered, reveal the remaining sides in reverse order on request, and offer the grades only after
the first side has been revealed. A reverse review SHALL be one review, and the session SHALL name the
direction on screen.

#### Scenario: The answer comes before the question

- **WHEN** the user studies the `rev` prompt of a card whose sides are a question and its answer
- **THEN** the answer is shown first, the question is revealed on the next request, and grading
  becomes available then

#### Scenario: The direction is visible

- **WHEN** any card is being reviewed in reverse
- **THEN** the screen says so, and a forward review of the same card does not

### Requirement: A session counts directions, not cards

The system SHALL count each prompt as one item in a queue, so a card enabled in both directions holds
two items, and the per-day cap SHALL count each prompt it admits.

#### Scenario: Both directions of one card are new

- **WHEN** a new two-sided card exists with reverse enabled
- **THEN** the new queue holds it twice, once per direction, and admitting both counts two against
  the cap

# Spec Delta — study-session

## ADDED Requirements

### Requirement: A diagram face hides its rectangles

The system SHALL, when reviewing a mask group, show the card's image with that group's rectangles
covered, SHALL uncover them on the user's reveal, and SHALL offer the grades only after the reveal. A
diagram review SHALL be one review, and the caption SHALL be shown when the card has one.

#### Scenario: The rectangle is covered until the reveal

- **WHEN** the `mask:1` prompt of a two-group diagram is reviewed
- **THEN** group 1's rectangles are covered and group 2's are visible, and one reveal uncovers group 1
  and offers the grades

#### Scenario: The caption is visible

- **WHEN** a diagram card with a caption is reviewed
- **THEN** the caption is on screen during the review

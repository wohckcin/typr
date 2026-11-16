# Spec Delta

## ADDED Requirements

### Requirement: The selected tile is a card, not a position

The grid SHALL keep the card in force by its identity. Changing the search text, choosing another row or
trashing a card SHALL NOT move the highlight to whichever card now occupies the same list position. When
the selected card leaves the shown list, no tile SHALL be highlighted as if the user had chosen another.

#### Scenario: A narrowing search keeps the selected card

- **WHEN** a card is selected and the search text narrows the list so a different card sits at the same
  position
- **THEN** the highlight stays on the selected card, and it disappears only if that card no longer matches

#### Scenario: Enter opens the highlighted card

- **WHEN** the list has been narrowed by a search and the user presses Enter
- **THEN** the editor opens for the card whose tile carries the highlight

#### Scenario: Delete trashes the highlighted card

- **WHEN** the user presses Delete after a search narrowed the list
- **THEN** the highlighted card is trashed, not the card that holds the same position in the full list

#### Scenario: Trashing selects a neighbour

- **WHEN** the highlighted card is trashed
- **THEN** the card that took its place in the shown list becomes selected, and trashing the last card
  selects the new last card

### Requirement: The grid is operable from the keyboard

The grid SHALL move its selection by card from the keyboard: the arrow keys SHALL move one card, Home and
End SHALL move to the first and last card shown, and Enter SHALL open the selected card. Movement SHALL
start from the selected card's position in the list as it is shown.

#### Scenario: Arrows follow the shown order

- **WHEN** a middle card is selected and an arrow key is pressed
- **THEN** the selection moves to the adjacent card in the shown list, and at either end it stays put

#### Scenario: Home and End reach the ends

- **WHEN** Home is pressed and then End
- **THEN** the first card shown is selected, then the last

#### Scenario: Movement starts from the shown list

- **WHEN** a search narrows the list and an arrow key is pressed
- **THEN** the selection moves from the selected card's position in the narrowed list, not from its old
  position

#### Scenario: An empty list takes no key

- **WHEN** the shown list holds no card and Enter or Delete is pressed
- **THEN** nothing opens and nothing is trashed

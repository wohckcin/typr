# card-grid Specification

## Purpose
What a deck looks like when it is not being read for its details: every card as a tile in a
wrapped grid, each tile showing the one face a reader needs to recognise the card, the schedule
state a reader needs to decide what to open, and the tags a reader needs to find it again.

## Requirements

### Requirement: A deck is drawn as a wrapped grid of card tiles

The system SHALL draw the cards of the selected deck as tiles that wrap into as many columns as the window
allows, so that a deck of many cards is read as a wall rather than a list. A tile SHALL be a body above a
footer strip, and the body SHALL take whatever height the tile is given after the footer.

#### Scenario: A narrow window still draws whole tiles

- **WHEN** the window is narrower than two tiles and the gaps between them
- **THEN** one tile per line is drawn at full width and no tile is clipped horizontally

### Requirement: A tile has no fixed width

No tile SHALL have a fixed width. The tiles on a line SHALL share the width available to them equally, one
gap apart, and the number of columns SHALL follow from that width rather than from a constant, so a line is
filled to its right edge with no ragged remainder.

#### Scenario: A wide window shows more columns

- **WHEN** the same deck is drawn at 700 and at 1485 points wide
- **THEN** the wider window shows more tiles per line, the same number of tiles overall, and every line filled
  to its right edge

#### Scenario: Each tile is exactly its share

- **WHEN** three tiles and two 16-point gaps fill 767 points
- **THEN** each tile is 245 points wide, and no tile carries a width of its own

### Requirement: A tile shows the first side of its card

The system SHALL render only the first side of a card on its tile, with the markdown of that side rendered
as it is in the preview, and SHALL NOT reveal any later side. Hidden text SHALL stay hidden on a tile: every
hidden span SHALL be drawn masked, and a numbered span SHALL carry its group number so the grid shows how
many prompts the card asks. A tile SHALL never show a hidden answer, whichever group asks for it.

#### Scenario: A two-sided card hides its answer

- **WHEN** a card holds the sides "front" and "back"
- **THEN** its tile shows "front" and does not contain the text "back"

#### Scenario: Hidden text stays hidden on the tile

- **WHEN** a card's first side holds `{{insulin}}`
- **THEN** its tile draws the span masked, and the word "insulin" is nowhere in the tile

#### Scenario: Every group is masked and named

- **WHEN** a card's first side holds `{{2::oxygen}}` and `{{1::air}}`
- **THEN** its tile draws both spans masked as `[2 … ]` and `[1 … ]`, and neither "oxygen" nor "air" appears

### Requirement: A tile states how many sides it hides

The system SHALL show the number of hidden sides on a tile whose card holds more than one side, and SHALL
show no such label on a card with a single side. The label SHALL read as a full-width strip at the bottom
of the tile's body, and SHALL read `1 hidden side` in the singular and `N hidden sides` otherwise.

#### Scenario: A two-sided card carries a hidden-side badge

- **WHEN** a two-sided card is drawn
- **THEN** its tile states that one side is hidden

#### Scenario: A note carries no label

- **WHEN** a card with one side is drawn
- **THEN** its tile draws no hidden-side strip

#### Scenario: The label counts in words

- **WHEN** a card holds three sides
- **THEN** its strip reads `2 hidden sides`

### Requirement: A tile states where its schedule stands

The system SHALL show a schedule state on every tile: a new card says it is new, and a card that has been
studied says its current interval in whole days, zero days included. A card that is due now SHALL be
distinguishable from a card that is not.

#### Scenario: A new card says so

- **WHEN** a card has never been graded
- **THEN** its tile reads as new rather than as an interval

#### Scenario: A studied card says its interval

- **WHEN** a card was graded remembered an hour ago and its graduation interval is ten minutes
- **THEN** its tile reports an interval of zero days, not that it is new

### Requirement: A tile carries its card's tags

The system SHALL draw the card's tags as chips on the tile, in the order they were set, and SHALL replace the
overflow with a count when they do not fit the tile's width.

#### Scenario: Tags beyond the tile's width are counted

- **WHEN** a card carries five tags and only two fit the tile
- **THEN** the tile shows those two chips and states that three more exist

### Requirement: An empty deck offers a tile to fill

The system SHALL draw a dashed placeholder tile in an empty deck that starts a new card, sized and placed as
the first tile of the grid, and SHALL show the binding that does the same from the keyboard.

#### Scenario: An empty deck is not a blank page

- **WHEN** a deck with no cards is selected
- **THEN** a dashed tile is drawn and pressing it starts a new card in that deck

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

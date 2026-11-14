## ADDED Requirements

### Requirement: A deck is drawn as a wrapped grid of card tiles

The system SHALL draw the cards of the selected deck as tiles that wrap into as many columns as the window
allows, each tile at least 233 points wide and at least 130 points tall, so that a deck of many cards is
read as a wall rather than a list.

#### Scenario: A wide window shows more columns

- **WHEN** the same deck is drawn at 845 and at 1485 points wide
- **THEN** the wider window shows more tiles per line and the same number of tiles overall

#### Scenario: A narrow window still draws whole tiles

- **WHEN** the window is narrower than two tiles and the gaps between them
- **THEN** one tile per line is drawn at full width and no tile is clipped horizontally

### Requirement: A tile shows the first side of its card

The system SHALL render only the first side of a card on its tile, with the markdown of that side rendered
as it is in the preview, and SHALL NOT reveal any later side.

#### Scenario: A two-sided card hides its answer

- **WHEN** a card holds the sides "front" and "back"
- **THEN** its tile shows "front" and does not contain the text "back"

### Requirement: A tile states how many sides it hides

The system SHALL show the number of hidden sides on a tile whose card holds more than one side, and SHALL
show no such badge on a card with a single side.

#### Scenario: A two-sided card carries a hidden-side badge

- **WHEN** a two-sided card is drawn
- **THEN** its tile states that one side is hidden

#### Scenario: A note carries no badge

- **WHEN** a card with one side is drawn
- **THEN** its tile shows no hidden-side badge

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

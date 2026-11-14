# Spec Delta — card-grid

## MODIFIED Requirements

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

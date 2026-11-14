# Card embeds Specification

## Purpose

Text can embed another card, so one note can show another's content where it is written, without copying it and without letting the two drift.

## Requirements

### Requirement: A card can show another card's content

The system SHALL treat `![[Name]]` in a card body as an embed of the card called `Name`,
and SHALL render that card's body where the embed appears, in the editor preview and in a
review face. The rendered embed SHALL be visually set apart from the surrounding text.

#### Scenario: An embed shows the other card
- **WHEN** a card body contains `![[Photosynthesis]]` and a card named `Photosynthesis`
  exists
- **THEN** that card's body is drawn in place of the embed, set apart from the text
  around it

#### Scenario: A missing card is named, not linked
- **WHEN** the embed names a card that does not exist
- **THEN** the name is drawn with a missing marker, and the rest of the card renders

### Requirement: An embed changes no schedule

An embed SHALL NOT create a prompt, SHALL NOT change a due date, and SHALL NOT count as a
side of the embedding card.

#### Scenario: Embedding does not queue anything
- **WHEN** a note embeds a two-sided card
- **THEN** both cards keep exactly the prompts they had before the embed

### Requirement: Embeds do not recurse

A card that embeds itself, and an embed inside an embedded card, SHALL stop after one
level and draw a marker naming the card rather than its content.

#### Scenario: Self-embed draws a marker
- **WHEN** a card embeds itself
- **THEN** the review face draws a marker naming the card and does not repeat the body

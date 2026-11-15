# visual-language Specification

## Purpose

Make every screen read as one application: one module of spacing, type, radius and surface tokens, cards
that look like objects, a review canvas that holds one thing, a sidebar that groups what the user has,
and an empty state wherever a screen has nothing to show — with the result checked against recorded
scene pictures rather than described.

## Requirements

### Requirement: Every screen draws with one token set

The system SHALL draw spacing, type sizes, radii, elevation and the focus ring from one module, SHALL
NOT use a numeric literal for those dimensions inside a screen, and SHALL keep the same token set in
light and dark themes.

#### Scenario: A screen uses the scale

- **WHEN** any screen's source is searched for a spacing or font size literal
- **THEN** none is found, because each is a named token from the visual language module

#### Scenario: Both themes share the tokens

- **WHEN** the theme is changed
- **THEN** every dimension stays identical and only the colours change

### Requirement: A card is one object on the page

The system SHALL render a card as a single surface with a radius, a border and one step of elevation,
SHALL place everything the card shows inside that surface, SHALL show its metadata as secondary text, and
SHALL show a hover and a focus state that do not move it. What a card shows is `card-grid`'s to decide.

#### Scenario: A tile reads as one card

- **WHEN** the library draws a card
- **THEN** its text, its badges and its chips share one border, and hovering changes only colour and
  shadow

#### Scenario: Focus is visible without motion

- **WHEN** a card is focused by the keyboard
- **THEN** a focus ring is drawn inside its bounds and no other element shifts

### Requirement: The review surface holds one thing

The system SHALL draw a review on a dimmed canvas with the card centred, SHALL separate the prompt from
the revealed answer by spacing and type weight rather than by a rule, SHALL keep the answer area the same
size before and after the reveal, and SHALL show the keyboard hints beneath the card.

#### Scenario: Revealing does not move the card

- **WHEN** the answer is revealed
- **THEN** the card's outer bounds are unchanged and the answer appears inside them

#### Scenario: The hints sit under the card

- **WHEN** a review is open
- **THEN** the bindings are drawn below the card, secondary, and never inside it

### Requirement: The sidebar groups what the user has

The system SHALL group the sidebar into Decks, Tags and Views under headings, SHALL show a count beside
each group and beside each deck, and SHALL keep the grouping while filtering.

#### Scenario: Groups are labelled and counted

- **WHEN** the library holds three decks, two tags and one view
- **THEN** the sidebar shows those three headings, each with its count

#### Scenario: An empty group is still named

- **WHEN** no view exists
- **THEN** the Views heading is drawn with a count of zero and an action to make one

### Requirement: Every empty screen says what to do

The system SHALL draw an empty state in place of an empty list or queue, SHALL name what is missing, and
SHALL offer the action that fills it.

#### Scenario: An empty library offers the first card

- **WHEN** the library holds no card
- **THEN** the list area shows an empty state whose action creates a card

#### Scenario: An empty queue offers a cram session

- **WHEN** nothing is due
- **THEN** the review entry point says so and offers a cram session instead

### Requirement: The appearance is checked against a baseline

The system SHALL render each scene in the test suite, SHALL compare it with a committed baseline image,
SHALL fail on a difference larger than the stated tolerance, and SHALL fail when a scene or a baseline is
missing.

#### Scenario: An unchanged screen matches

- **WHEN** a scene is rendered and nothing changed
- **THEN** its baseline comparison passes

#### Scenario: A changed screen fails

- **WHEN** a padding token changes
- **THEN** at least one baseline comparison fails and names the scene

#### Scenario: A new scene needs a baseline

- **WHEN** a scene is added without a baseline image
- **THEN** the comparison fails rather than skipping it

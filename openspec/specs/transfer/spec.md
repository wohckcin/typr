# transfer Specification

## Purpose
Owns moving material in and out of a library: the library itself as one file that reopens exactly, a Mochi export to bring in, and the text formats people read and edit outside the app. An import either lands whole or writes nothing, and an export never disturbs the library it came from.

## Requirements

### Requirement: A library exports as one file that reopens

The system SHALL export the whole library as one file that opens as a library on another machine,
including decks, cards, fields, masks, tags, images and every review record, and SHALL NOT alter the
exporting library.

#### Scenario: The round trip is lossless

- **WHEN** a library holding two decks, twenty cards, three images and fifty review records is exported
  and the export is opened
- **THEN** every card, image, prompt, due date and record is present and equal

#### Scenario: Exporting does not write to the library

- **WHEN** a library is exported
- **THEN** its own file's modification time and schema version are unchanged

### Requirement: A Mochi export can be imported

The system SHALL import a Mochi export into the current library, SHALL create its decks with their
nesting and names, SHALL create its cards with their text, templates, fields and tags, SHALL copy its
images into the attachments folder, and SHALL import each card's review history as records of the
prompts the card's variants map to.

#### Scenario: Decks and cards arrive whole

- **WHEN** a Mochi export of three nested decks and forty cards is imported
- **THEN** the three decks exist with their nesting, each card sits in its deck, and the library holds
  forty more cards

#### Scenario: History arrives with its grades

- **WHEN** an imported card has ten Mochi reviews, mixed remembered and forgot
- **THEN** the card holds ten records with the matching grades and times, and its due date follows from
  its most recent record

#### Scenario: A variant becomes a prompt

- **WHEN** an imported card was reviewed in both directions in Mochi
- **THEN** it holds both `fwd` and `rev` prompts, each with its own imported records

### Requirement: An unrecognised archive is refused whole

The system SHALL read an import file's shape before writing anything, SHALL refuse a file it cannot
recognise with a message naming what it found, and SHALL leave the library unchanged when it refuses.

#### Scenario: A truncated archive imports nothing

- **WHEN** a Mochi export is truncated mid-file and imported
- **THEN** the library holds exactly the cards it held before, and the failure names the unreadable part

#### Scenario: A newer export version is refused

- **WHEN** the archive declares a format version this build does not know
- **THEN** the import is refused with that version in the message

### Requirement: Text formats import and export

The system SHALL export a deck's cards as CSV with a documented header and as Markdown with the
separator and hidden-text syntax preserved, and SHALL import a CSV whose header names the columns into a
chosen deck, mapping sides, tags and fields.

#### Scenario: A CSV round trip keeps the sides

- **WHEN** a two-sided card with two tags is exported to CSV and imported into the same deck
- **THEN** the imported card has the same sides and tags

#### Scenario: Markdown keeps the syntax

- **WHEN** a card with a separator and a hidden-text group is exported to Markdown
- **THEN** the file holds `---` and `{{1::…}}` verbatim, and importing it reproduces the same prompts

#### Scenario: A CSV without a header is refused

- **WHEN** the chosen file's first line is not the expected header
- **THEN** nothing is imported and the failure names the missing columns

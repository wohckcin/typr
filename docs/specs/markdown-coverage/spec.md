# markdown-coverage Specification

## Purpose

Which markdown constructs a card's text renders, how a construct the renderer cannot draw keeps its text, and
where the one list of constructs lives.

## Requirements

### Requirement: Highlighted text renders as highlighted

The system SHALL render text between double equals signs as highlighted text, SHALL NOT show the markers, and
SHALL NOT treat `==` inside a code span or a fenced block as a highlight.

#### Scenario: A highlighted phrase loses its markers

- **WHEN** a card holds `==the Krebs cycle==`
- **THEN** the preview and the review show the Krebs cycle highlighted and show no equals signs

#### Scenario: Code is left alone

- **WHEN** a card holds `` `a == b` ``
- **THEN** the code is shown as written, markers included, and nothing is highlighted

### Requirement: Superscript is read and drawn apart from the run

The system SHALL read text between carets as superscript, SHALL NOT show the markers, and SHALL keep a subscript
(`H~2~0`) reading the way the renderer already reads it.

#### Scenario: An ordinal

- **WHEN** a card holds `20^th^ century`
- **THEN** "th" is drawn smaller and raised, and no caret appears

#### Scenario: A renderer that cannot shift text

- **WHEN** the drawing surface cannot raise a run
- **THEN** the construct is listed as degraded in the supported list, and the text is still shown without its
  markers

### Requirement: A ruby gloss draws its reading with its base

The system SHALL read `{base}(reading)` as a base with its reading set above it, SHALL NOT show the braces or the
parentheses, and SHALL leave text without that shape untouched.

#### Scenario: A Japanese gloss

- **WHEN** a card holds a base inside braces followed by its reading in parentheses
- **THEN** the base is drawn with the reading above it and no marker punctuation appears

#### Scenario: Parentheses alone are not a gloss

- **WHEN** a card holds `mitosis (cell division)`
- **THEN** it renders as written, since no braces surround the base

### Requirement: A construct the renderer does not know keeps its text

The system SHALL show any syntax it does not read as written, and SHALL NOT drop it.

#### Scenario: A definition list

- **WHEN** a card holds a term line followed by a definition line
- **THEN** both lines are shown as written, since the renderer reads no definition list

#### Scenario: Raw HTML

- **WHEN** a card holds an HTML tag
- **THEN** the tag is shown as written rather than interpreted

### Requirement: One list says what every construct does

The system SHALL keep one list naming each markdown construct with its support — rendered, degraded or
unsupported — and SHALL have a renderer test for every entry, so a construct cannot be handled by accident and
left out of the list.

#### Scenario: A construct gains handling

- **WHEN** a construct's handling changes
- **THEN** its row in the list changes with it, and the test named by that row fails until it does

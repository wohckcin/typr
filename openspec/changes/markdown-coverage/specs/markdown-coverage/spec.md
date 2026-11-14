## ADDED Requirements

### Requirement: Highlighted text renders as highlighted

The system SHALL render text between double equals signs as highlighted text, SHALL NOT include the markers in
the rendered output, and SHALL NOT treat `==` inside a code span or a fenced block as a highlight.

#### Scenario: A highlighted phrase loses its markers

- **WHEN** a card holds `==the Krebs cycle==`
- **THEN** the preview and the review show the Krebs cycle highlighted and show no equals signs

#### Scenario: Code is left alone

- **WHEN** a card holds `` `a == b` ``
- **THEN** the code is shown as written, markers included, and nothing is highlighted

### Requirement: Superscript and subscript render as such

The system SHALL render text between carets as superscript and text between tildes as subscript, and SHALL keep
strikethrough working for double tildes.

#### Scenario: An ordinal renders raised

- **WHEN** a card holds `20^th^ century`
- **THEN** the "th" is rendered raised and the markers are not shown

#### Scenario: A chemical formula renders lowered

- **WHEN** a card holds `H~2~0`
- **THEN** the numeral is rendered lowered and the markers are not shown

### Requirement: Definition lists render as a term and its definition

The system SHALL render a term followed by a definition line as a definition list, with the term distinguished
from its definition, and SHALL keep consecutive definitions grouped under their term.

#### Scenario: A term with two definitions

- **WHEN** a card holds a term line followed by two definition lines
- **THEN** the term is rendered once and both definitions are rendered under it

### Requirement: A ruby gloss renders its reading over the base

The system SHALL render `{base}(reading)` as the base text with the reading set above it, SHALL NOT show the
braces or the parentheses, and SHALL leave text without the brace-and-parenthesis shape untouched.

#### Scenario: A Japanese gloss

- **WHEN** a card holds the base characters for a word followed by its reading in parentheses inside braces
- **THEN** the base is shown with the reading above it and no punctuation from the markers appears

#### Scenario: Parentheses alone are not a gloss

- **WHEN** a card holds `mitosis (cell division)`
- **THEN** it renders as written, since no braces surround the base

### Requirement: Math is preserved and set apart

The system SHALL keep math written between dollar signs and between double dollar signs in the rendered output,
SHALL distinguish it from surrounding prose, and SHALL NOT drop it or leave its markers attached to words. The
system SHALL NOT claim to typeset math.

#### Scenario: Inline math stays readable

- **WHEN** a card holds `the area is $\\pi r^2$` 
- **THEN** the math is shown as its own run, set apart from the prose, with no dollar signs lost from the page

#### Scenario: Display math stays readable

- **WHEN** a card holds math between double dollar signs on its own line
- **THEN** it is shown on its own line and set apart

### Requirement: Unknown constructs degrade to their literal text

The system SHALL render any construct the renderer does not implement as the text that was written, SHALL NOT
drop it, and SHALL keep the list of supported constructs in one place that the renderer and the tests share.

#### Scenario: An unsupported badge does not vanish

- **WHEN** a card holds syntax the renderer does not implement
- **THEN** that text appears in the output exactly as written

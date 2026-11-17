## ADDED Requirements

### Requirement: Highlighted text renders as highlighted


#### Scenario: A highlighted phrase loses its markers

- **WHEN** a card holds `==the Krebs cycle==`
- **THEN** the preview and the review show the Krebs cycle highlighted and show no equals signs

#### Scenario: Code is left alone

- **WHEN** a card holds `` `a == b` ``
- **THEN** the code is shown as written, markers included, and nothing is highlighted




- **WHEN** a card holds `20^th^ century`


- **WHEN** a card holds `H~2~0`





#### Scenario: A Japanese gloss


#### Scenario: Parentheses alone are not a gloss

- **WHEN** a card holds `mitosis (cell division)`
- **THEN** it renders as written, since no braces surround the base











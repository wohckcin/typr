# Spec Delta — card-editor

## ADDED Requirements

### Requirement: Hidden text is written inline and previewed masked

The editor SHALL accept hidden text in the card's markdown as typed, SHALL preview it masked in the
rendered pane, and SHALL show which group a mask belongs to. A card holding hidden text SHALL ignore
the three-hyphen separator, so the preview shows one face rather than two sides.

#### Scenario: The preview shows the mask, not the answer

- **WHEN** the user types `Our lungs pull {{2::oxygen}} from the {{1::air}}.`
- **THEN** the preview shows the sentence with both spans masked and reports two prompts

#### Scenario: Unnumbered spans are one prompt

- **WHEN** the user types `The hormone {{insulin}} is produced by the {{pancreas}}.`
- **THEN** the preview masks both spans and reports one prompt

#### Scenario: A separator stops splitting a hidden-text card

- **WHEN** a card holds hidden text and a line of three hyphens
- **THEN** the preview reports one face and the separator is shown as ordinary content

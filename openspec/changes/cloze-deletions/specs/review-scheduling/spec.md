# Spec Delta — review-scheduling

## MODIFIED Requirements

## REMOVED Requirements

### Requirement: Each card has at most one prompt

**Reason**: the card may now carry one prompt per review variant, so "at most one" is false whenever a
card holds two hidden-text groups.

**Migration**: no data change. Every prominent card keeps the single `fwd` prompt it already had; the
new keys are only created for cards that hold hidden text.

## ADDED Requirements

### Requirement: Each card has one prompt per review variant

The system SHALL track scheduling state per prompt, identified by a card and a key, SHALL hold exactly
one prompt per review variant a card carries, and SHALL hold none for a card with no variant. The keys
are `fwd` for two or more sides, `cloze:<n>` per numbered hidden-text group, and `cloze:0` for unnumbered
hidden text. Each variant SHALL keep its own schedule and history.

#### Scenario: One prompt per two-sided card

- **WHEN** a two-sided card is created
- **THEN** exactly one prompt exists for it, and grading that card updates that prompt's state

#### Scenario: A single-sided card has no prompt

- **WHEN** a card with one side is saved and holds no hidden text
- **THEN** no prompt is created and the card never appears in a study queue

#### Scenario: Two groups, two schedules

- **WHEN** a card holds `{{1::insulin}}` and `{{2::pancreas}}`
- **THEN** it has prompts `cloze:1` and `cloze:2`, and grading one leaves the other untouched

#### Scenario: Sides and groups never coexist

- **WHEN** a card with a three-hyphen separator also holds hidden text
- **THEN** the card has the hidden-text prompts and no `fwd` prompt

## ADDED Requirements

### Requirement: Hidden text is parsed outside code

The system SHALL read `{{text}}` and `{{n::text}}` as hidden text only when they appear outside a
fenced code block and outside inline code, SHALL treat an unclosed brace as ordinary text, and SHALL
accept any positive integer as a group number.

#### Scenario: Documented markdown is not hidden text

- **WHEN** a card explains the syntax inside a fenced block, such as `{{example}}`
- **THEN** that text is not hidden and creates no prompt

#### Scenario: An unclosed brace is text

- **WHEN** a card contains `half {{open`
- **THEN** it holds no hidden text and behaves as a card without it

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
hidden text. Each variant SHALL keep its own schedule and history, and a variant the card no longer
holds SHALL leave the queues until the text asks for it again.

#### Scenario: One prompt per two-sided card

- **WHEN** a two-sided card is created
- **THEN** exactly one prompt exists for it, and grading that card updates that prompt's state

#### Scenario: A single-sided text card has no prompt

- **WHEN** a text card with one side is saved and holds no hidden text
- **THEN** no prompt is created and no queue of it selects the card

#### Scenario: Two groups, two schedules

- **WHEN** a card holds `{{1::insulin}}` and `{{2::pancreas}}`
- **THEN** it has prompts `cloze:1` and `cloze:2`, and grading one leaves the other untouched

#### Scenario: A group whose spans are gone leaves the queue but keeps its records

- **WHEN** a card that held `{{1::insulin}}` and `{{2::pancreas}}` is saved holding only the first group
- **THEN** `cloze:2` is in no queue and counts against neither the cap nor the deck's due count, its
  records stay, and saving the second group back puts it in the queue with the schedule it kept

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

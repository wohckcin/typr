# review-scheduling Specification

## Purpose
Owns the memory model: what the scheduler tracks, how a grade changes a card's due date, and the history that every future scheduling algorithm depends on.

## Requirements

### Requirement: Grades are binary

The system SHALL accept exactly two grades for a prompt: `remembered` and `forgot`. There SHALL be no third grade and no self-reported confidence.

#### Scenario: Only two grades are offered

- **WHEN** the user is grading a prompt
- **THEN** the available choices are remembered and forgot

### Requirement: Every grade is recorded with its phase

The system SHALL append a record for every grade, containing the card, the prompt key, the phase, the time, the grade, the interval assigned and the resulting due date. The phase SHALL be `learn` while the card still sits in the new-card queue, `review` for a scheduled prompt including the grade that graduates a card, and `relearn` inside a relearning step. Records SHALL NEVER be modified or deleted individually; only a prompt's whole history is removable.

#### Scenario: A forgotten new card is still recorded

- **WHEN** the user grades `forgot` while learning a new card
- **THEN** a record with phase `learn` is appended, and the card's schedule is otherwise unchanged

#### Scenario: History is append-only

- **WHEN** the user grades the same prompt ten times
- **THEN** ten records exist in grade order and none earlier record differs from when it was written

### Requirement: Learning cycles never change the schedule

The system SHALL exclude `learn`-phase records from interval arithmetic and from statistics that count reviews, and SHALL record the `remembered` grade that graduates a card with phase `review`. A `learn`-phase `forgot` SHALL leave scheduling state unchanged. When a prompt's scheduling state is recomputed from history, it SHALL be derivable from the records alone.

#### Scenario: Learning cycles do not count as reviews

- **WHEN** the user grades `forgot` twice and then `remembered` while learning a card, then views the review count for that card
- **THEN** the reported review count is one, not three

#### Scenario: Due dates are recomputed identically from history

- **WHEN** a prompt's scheduling state is recomputed from its stored records
- **THEN** the resulting due date equals the stored one

### Requirement: Remembered lengthens the interval

The system SHALL enter a prompt in two steps: a `remembered` grade in the `learn` phase SHALL leave the learning queue and set the due date to ten minutes later, the following `remembered` SHALL set a one-day interval, and each later `remembered` SHALL multiply the interval by 2.5, capped at 365 days.

#### Scenario: First remembered keeps the card due today

- **WHEN** the user grades `remembered` while learning a new card
- **THEN** the prompt leaves the learning queue and its due date is ten minutes later

#### Scenario: Second success sets a one-day interval

- **WHEN** that prompt, due ten minutes later, is graded `remembered`
- **THEN** its next interval is one day

#### Scenario: Repeated successes stretch the interval

- **WHEN** a prompt with a 10-day interval is graded `remembered`
- **THEN** its next interval is 25 days

#### Scenario: The interval is capped

- **WHEN** a prompt with a 200-day interval is graded `remembered`
- **THEN** its next interval is 365 days, not 500

### Requirement: Forgot shortens the interval and is retried soon

The system SHALL, on `forgot` during `review` or `relearn`, record the grade with the corresponding phase, schedule the prompt ten minutes later in a relearning step, and set a one-day interval on the `remembered` that follows that step.

#### Scenario: Failure returns the prompt the same session

- **WHEN** a due prompt is graded `forgot`
- **THEN** it is recorded with phase `relearn` and is due again ten minutes later

#### Scenario: Success after a failure starts short

- **WHEN** a prompt in a relearning step is graded `remembered`
- **THEN** its next interval is one day

### Requirement: Resetting history returns a prompt to new

The system SHALL allow resetting one prompt's history, SHALL remove all its records including `learn` ones, and SHALL leave the prompt with no scheduling state so that it is treated as never studied.

#### Scenario: Reset makes the card new again

- **WHEN** the user resets the review history of a studied card
- **THEN** the card appears in the new-card queue and has no due date

### Requirement: The due queue selects and orders prompts

The system SHALL select the prompts of studyable cards whose due date is at or before the current moment, SHALL order them so that the most overdue day comes first, where a prompt's overdue amount is counted in whole study days, and SHALL shuffle the prompts within each group of equal overdue days. Prompts with no scheduling state SHALL NOT be selected.

#### Scenario: Overdue prompts come first

- **WHEN** three prompts are due and one became due a week ago
- **THEN** that prompt is ordered before the other two

#### Scenario: Equal urgency is shuffled

- **WHEN** two prompts became due on the same study day
- **THEN** their relative order is decided by the shuffle, not by their due timestamps

#### Scenario: Trashed prompts stay out

- **WHEN** a card is in the trash
- **THEN** its prompts are not selected

#### Scenario: Future prompts are excluded

- **WHEN** a prompt's due date is tomorrow
- **THEN** it is not selected for today's due queue

### Requirement: A card's review count and history are visible

The system SHALL show, for one card, the number of reviews it has had and the list of its records with their phase, grade, time and due date. The count SHALL exclude `learn`-phase records and SHALL include every other record.

#### Scenario: Learning cycles are excluded from the count

- **WHEN** a card has three `learn`-phase records and two `review`-phase records
- **THEN** its review count is two and its history lists all five records with their phases

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

### Requirement: A reverse review is its own prompt

The system SHALL give every card that carries a review variant a second prompt keyed `rev`, SHALL keep
its phase, interval, due date and history separate from every other prompt of that card, and SHALL
grade it from the same two binary grades.

#### Scenario: The two directions are counted separately

- **WHEN** a two-sided card is reviewed forward and remembered, then reviewed in reverse
- **THEN** `fwd` holds one record and `rev` holds none, and the card's review count is one

#### Scenario: A reverse grade schedules only itself

- **WHEN** the `rev` prompt is graded remembered while `fwd` is due
- **THEN** `rev` moves to its new interval and `fwd` keeps its own due date

### Requirement: Reverse is enabled per card, deck or library

The system SHALL resolve whether a card is reviewed in reverse from the card's own setting, else the
nearest ancestor deck that sets one, else the library default, and SHALL default to off. Changing any
of those settings SHALL affect future reviews only: no prompt row, record or interval SHALL be deleted
or recomputed, and a `rev` prompt that stops being enabled SHALL cease to queue while keeping its
history.

#### Scenario: A card setting beats its deck

- **WHEN** a card inside a deck that enables reverse disables it for itself
- **THEN** only `fwd` queues for that card, and the card's deck still enables reverse for its siblings

#### Scenario: Turning reverse off keeps the history

- **WHEN** the library default was on, a card has been reviewed in reverse twice, and the default is
  switched off
- **THEN** the `rev` prompt disappears from both queues and its two records remain readable

#### Scenario: Enabling reverse on a deck reaches its descendants

- **WHEN** reverse is enabled on a deck that holds nested decks with cards
- **THEN** the cards in those nested decks queue in reverse too, unless one of them sets its own value

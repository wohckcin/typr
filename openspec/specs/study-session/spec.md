# study-session Specification

## Purpose
Defines the act of studying: which prompts a session contains, the order the user moves through them, how a card's sides are revealed, and how the session is driven without a mouse.

## Requirements

### Requirement: Two distinct study queues

The system SHALL offer a new-card queue containing the prompts of studyable cards that have no scheduling state and that the daily cap has not deferred, and a due queue containing the prompts of studyable cards that the schedule has made due. The two SHALL be entered separately and SHALL NOT be mixed into one queue.

#### Scenario: A new card is only in the new queue

- **WHEN** a two-sided card has been created but never studied
- **THEN** it appears in the new-card queue and not in the due queue

#### Scenario: A note is in neither queue

- **WHEN** a one-sided card exists
- **THEN** it appears in neither queue

#### Scenario: A learned card only enters the due queue

- **WHEN** a card has been graded `remembered` while learning and is not yet due
- **THEN** it appears in neither queue until its due date arrives

### Requirement: A session is scoped to a deck

The system SHALL start a session for the whole library or for one deck, and a deck session SHALL restrict both queues to the cards of that deck and its descendants.

#### Scenario: Deck session excludes sibling decks

- **WHEN** the user starts a due session while "Verbs" is selected
- **THEN** due prompts from outside "Verbs" and its descendants are not included

### Requirement: Sides are revealed in order within one review

The system SHALL show one face of the reviewed prompt at a time, SHALL reveal each later face only on
the user's request and in order, and SHALL offer the grade only after the last face has been revealed.
This SHALL hold in both queues, and revealing faces SHALL count as one review, not several. A face SHALL
carry the text to show while hidden and the text to show once revealed.

#### Scenario: Three-sided card is one review

- **WHEN** the user studies a card with three sides
- **THEN** the sides are revealed in order and a single grade is recorded for the prompt

#### Scenario: Grading is unavailable before the last side

- **WHEN** the first side of a two-sided card is showing
- **THEN** the remembered and forgot actions are not available until the second side is revealed

#### Scenario: One hidden-text face unmasks

- **WHEN** the prompt `cloze:1` is reviewed
- **THEN** the sentence is shown with that group masked, one reveal unmasks it, and the grade is
  offered straight away

### Requirement: Forgot in the learning queue cycles without rescheduling

The system SHALL, on `forgot` while learning a new card, move the card to the end of the remaining new-card queue in this session and SHALL NOT give it a due date.

#### Scenario: A forgotten new card returns later in the session

- **WHEN** the user grades `forgot` on a new card while three others remain
- **THEN** the three others are shown first and that card is offered again afterwards

### Requirement: The session is fully keyboard-operable

The system SHALL expose a keyboard action to reveal the next side, to grade remembered, to grade forgot, and to move forward and backward within the queue, and SHALL display the binding for each action in the session. Moving backward SHALL show an earlier card read-only and SHALL NOT accept a second grade; moving forward past a card without grading it SHALL leave that card in the queue.

#### Scenario: A session completes without a mouse

- **WHEN** the user works through a queue using only the keyboard
- **THEN** every card can be revealed and graded and the session reaches its end

#### Scenario: Backward does not grade again

- **WHEN** the user moves backward to a card that was already graded and then forward again
- **THEN** exactly one grade exists for that card

### Requirement: New cards per day can be capped

The system SHALL enforce a limit on how many new cards are learned per study day, SHALL apply the limit to the whole library across sessions, and SHALL treat cards deferred by the limit as still new. A card SHALL count against a study day when its earliest `review`-phase record falls on that day.

#### Scenario: The cap stops the queue early

- **WHEN** the limit is 20 and 20 cards have already been learned today
- **THEN** the new-card queue does not offer another card today and those cards remain new tomorrow

### Requirement: Leaving a session preserves what was graded

The system SHALL commit each grade as it is taken, and SHALL discard only the position within the queue, so that a session abandoned halfway resumes with the graded cards excluded and the remaining ones still queued.

#### Scenario: Abandoned session resumes

- **WHEN** the user grades five of twelve due cards and quits the app
- **THEN** the next session contains the seven ungraded ones and the five already-graded ones only if they became due again

### Requirement: The queue order is re-rollable

The system SHALL shuffle the due queue, SHALL allow the user to shuffle it again before or during a session, and SHALL NOT change any prompt's scheduling state when doing so. A reshuffle SHALL reorder prompts within each group of equal overdue days and SHALL leave the order between groups unchanged.

#### Scenario: Reshuffling changes order but not schedules

- **WHEN** the user reshuffles a due queue holding at least two prompts with the same overdue day
- **THEN** those prompts' relative order changes and every prompt's due date is unchanged

### Requirement: Prompts falling due mid-session join the running queue

The system SHALL offer a prompt whose due date passes during a due session within that same session, SHALL end a due session once no prompt remains due, and SHALL let the user end a session at any time even when prompts remain.

#### Scenario: A relearning step returns before the session ends

- **WHEN** a prompt is graded `forgot` and becomes due ten minutes later while the session is still running
- **THEN** it is offered again in that session

### Requirement: A finished session reports what happened

The system SHALL show, at the end of a session, how many grades of each kind were recorded in that session and how many prompts are due at that moment, and SHALL offer a way back to the library.

#### Scenario: Session summary after study

- **WHEN** the user grades the last card in a queue
- **THEN** a summary reports the remembered and forgot grades recorded in the session, the number of prompts due at that moment, and a way back to the library

### Requirement: A failed write is visible and leaves the card answerable

The system SHALL show why a write or query did not happen, SHALL leave the current card answerable after a grade is refused, and SHALL NOT advance a session on an answer the store did not confirm.

#### Scenario: A refused grade stays on screen

- **WHEN** a grade is sent for a card that has no prompt and the store refuses it
- **THEN** the card stays on screen, the reason is shown, and no grade is recorded

#### Scenario: A grade that could not be sent is not waited on

- **WHEN** the library worker has closed and a grade cannot be sent
- **THEN** the failure is reported and the session does not wait for an answer

### Requirement: A reverse review shows the last side first

The system SHALL, when reviewing the `rev` prompt of a card, show the card's last side as the face to
be answered, reveal the remaining sides in reverse order on request, and offer the grades only after
the first side has been revealed. A reverse review SHALL be one review, and the session SHALL name the
direction on screen.

#### Scenario: The answer comes before the question

- **WHEN** the user studies the `rev` prompt of a card whose sides are a question and its answer
- **THEN** the answer is shown first, the question is revealed on the next request, and grading
  becomes available then

#### Scenario: The direction is visible

- **WHEN** any card is being reviewed in reverse
- **THEN** the screen says so, and a forward review of the same card does not

### Requirement: A session queues directions and caps cards

The system SHALL count each prompt as one item in a queue, so a card enabled in both directions holds
two items, while the per-day cap SHALL count each card once however many directions it holds.

#### Scenario: Both directions of one card are new

- **WHEN** a new two-sided card exists with reverse enabled
- **THEN** the new queue holds it twice, once per direction, and it counts once against the cap

#### Scenario: A card learned in both directions fills one slot

- **WHEN** a card asked both ways graduates and the library counts what was learned today
- **THEN** the day counts one card, not two

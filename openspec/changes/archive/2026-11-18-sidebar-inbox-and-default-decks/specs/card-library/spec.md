# Spec Delta

## ADDED Requirements

### Requirement: A new library starts with Flashcards and Notes

When the app opens a library that holds no deck, live or archived, it SHALL create two top-level
decks, `Flashcards` then `Notes`, before the first frame is drawn. A library that holds any deck
SHALL be left as it is, and the app SHALL NOT recreate either deck after the user renames or archives it.

#### Scenario: First open makes the starter decks
- **WHEN** the app opens for the first time with no library file
- **THEN** the library holds exactly two decks, `Flashcards` and `Notes`, both top-level

#### Scenario: An existing library is untouched
- **WHEN** the app opens a library that holds one deck named `Default`
- **THEN** the library still holds exactly that one deck, and no `Flashcards` or `Notes` deck is made

#### Scenario: A renamed starter deck stays renamed
- **WHEN** the user renames `Notes` to `Reading` and relaunches the app
- **THEN** the library holds `Flashcards` and `Reading`, and no new `Notes` deck

### Requirement: The trash screen lists what can be restored

The trash screen SHALL list every trashed card with its preview and its deck, most recently trashed
first. Each row SHALL offer Restore. A trash with no card SHALL say that the trash is empty.

#### Scenario: Trashed cards are listed
- **WHEN** two cards are trashed, `A` before `B`
- **THEN** the trash screen lists `B` then `A`, each with its deck's name

#### Scenario: Restore returns the card
- **WHEN** the user presses Restore on a trashed card
- **THEN** the card leaves the trash list and is back in its deck with its scheduling unchanged

#### Scenario: An empty trash says so
- **WHEN** the trash screen is opened with no trashed card
- **THEN** the screen states that the trash is empty and offers no Empty trash action

### Requirement: Emptying the trash asks first

The trash screen SHALL offer Empty trash only when the trash holds a card. Pressing it SHALL ask for
confirmation on the screen, naming how many cards will be destroyed. Only the confirming press SHALL
destroy them, and cancelling SHALL leave the trash as it was.

#### Scenario: Confirming empties the trash
- **WHEN** the trash holds three cards and the user presses Empty trash and then confirms
- **THEN** the three cards and their review history are gone and the screen states the trash is empty

#### Scenario: Cancelling keeps the cards
- **WHEN** the user presses Empty trash and then cancels
- **THEN** every trashed card is still listed

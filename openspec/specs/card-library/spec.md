# card-library Specification

## Purpose
Owns the user's durable study material — the decks, cards and tags that exist independently of any review — and the operations that organize, find and retire it.

## Requirements

### Requirement: Decks form a hierarchy

The system SHALL store decks in a parent/child hierarchy with one root deck that has no parent, and SHALL allow any deck to nest under any other deck without a depth limit. Two decks under the same parent SHALL NOT share a name, and decks under different parents MAY share one.

#### Scenario: Creating a nested deck

- **WHEN** the user creates a deck named "Verbs" inside a deck named "Spanish"
- **THEN** "Verbs" appears as a child of "Spanish" and its cards are not listed under "Spanish" directly

#### Scenario: Duplicate names under one parent are refused

- **WHEN** the user creates a second deck named "Verbs" under "Spanish"
- **THEN** the creation is refused and the hierarchy is unchanged

#### Scenario: Rejecting a cycle

- **WHEN** the user moves a deck into one of its own descendants
- **THEN** the move is refused and the hierarchy is unchanged

### Requirement: Every card belongs to exactly one deck

The system SHALL require each card to belong to one deck at creation and SHALL move the card, together with its review history, when it is reassigned. Moving a card SHALL NOT reset its scheduling state.

#### Scenario: Moving a card between decks

- **WHEN** the user moves a card that has been reviewed from "Spanish" to "Verbs"
- **THEN** the card appears in "Verbs" and its due date is unchanged

### Requirement: Studyable cards exclude archived and trashed material

The system SHALL treat a card as studyable only when the card is not archived, not in the trash, and no deck holding it — its own deck or any ancestor — is archived. Study queues SHALL contain only studyable cards, and the card list SHALL show a trashed card only in the trash view.

#### Scenario: Archiving a deck removes its cards from study

- **WHEN** a deck holding a card that is due today is archived
- **THEN** that card is absent from the due queue and its due date is preserved

#### Scenario: A trashed card is not studyable

- **WHEN** a card is moved to the trash
- **THEN** it appears in no study queue and not in its deck's card list

### Requirement: Cards are listed with their content searchable

The system SHALL list the cards of a selected deck and its descendants and SHALL provide a text search over card content and tags. Search SHALL match case-insensitively and SHALL ignore diacritics, so that "cancion" finds "canción".

#### Scenario: Searching ignores accents and case

- **WHEN** the user searches "cancion" and a card contains "Canción"
- **THEN** that card appears in the results

#### Scenario: Search scope follows the selected deck

- **WHEN** the user searches while a deck is selected
- **THEN** results are limited to cards in that deck and its descendants

### Requirement: Tags classify cards

The system SHALL support zero or more tags per card, SHALL allow filtering the card list by tag, and SHALL treat tags as case-insensitive for matching while preserving the casing first used for that tag across the library. Tags SHALL be set on the card rather than parsed out of its text, so `#` in a heading or inside a side is content.

#### Scenario: Filtering by tag

- **WHEN** the user filters by "grammar" and a card is tagged "Grammar"
- **THEN** that card appears in the filtered list

#### Scenario: A heading is not a tag

- **WHEN** a card's text starts with `# Grammar`
- **THEN** the card has no tag and is not matched by a "grammar" tag filter

### Requirement: Archiving removes a card from study without deleting it

The system SHALL allow a card or a whole deck to be archived. An archived card SHALL NOT appear in
any study queue, SHALL retain its content and review history, and SHALL remain visible in an archived
view from which it can be restored. A deck SHALL be restorable from the same archived view, and
restoring it SHALL return the cards it holds to the views and queues they belong in without changing
their scheduling state.

#### Scenario: Archived card leaves the due queue

- **WHEN** a card that is due today is archived
- **THEN** it is omitted from due today and its due date is preserved for the day it is restored

#### Scenario: A card under an archived deck is reachable and the deck comes back

- **WHEN** a deck holding a scheduled card is archived, and the user then restores that deck
- **THEN** the card is visible in the archived view while the deck is archived, and after the deck is
  restored the card is in the active view again with its due date unchanged

### Requirement: Trash is reversible until emptied

The system SHALL move deleted cards to a trash area rather than destroying them, SHALL allow restoring a card from trash with its review history intact, and SHALL destroy trashed cards, with their review history, only when the user explicitly empties the trash.

#### Scenario: Restoring from trash

- **WHEN** the user deletes a card and later restores it
- **THEN** the card returns to its original deck with its scheduling state and history unchanged

#### Scenario: Emptying trash is irreversible

- **WHEN** the user empties the trash
- **THEN** the trashed cards and their review history are gone and cannot be restored

### Requirement: Library data survives restarts in one local database

The system SHALL persist all decks, cards, tags, settings and review history in one SQLite database in the platform's application data directory for this app, SHALL create that database on first launch, and SHALL make every saved change durable with no further step.

#### Scenario: Restart preserves edits

- **WHEN** the user creates a card, closes the app, and launches it again
- **THEN** the card is present with the same content, deck and scheduling state

#### Scenario: Unreadable store is reported, not silently discarded

- **WHEN** the store file exists but cannot be opened or is not a database this app wrote
- **THEN** the app reports the failure with the file path and does not overwrite it

### Requirement: The library says which direction a card is waiting on

The system SHALL show, for each card that is enabled in reverse, that both directions are in play, and
SHALL offer a control that toggles reverse for that one card without opening it.

#### Scenario: A card shows its directions

- **WHEN** a two-sided card is enabled in reverse and its tile is drawn
- **THEN** the tile says the card is asked in both directions, and a card without reverse does not

#### Scenario: The tile toggles one card

- **WHEN** the user toggles reverse off from the card's tile
- **THEN** that card stops queueing in reverse and its deck setting is unchanged

### Requirement: A deck carries its own review settings

The system SHALL let a deck enable or disable reverse for itself, SHALL let a deck inherit the setting
from its parent, and SHALL show which of the three states a deck is in.

#### Scenario: A deck is switched to inherit

- **WHEN** a deck that had reverse enabled is set back to inherit while its parent has reverse off
- **THEN** the deck's cards stop queueing in reverse and the deck reports that it inherits

#### Scenario: The library default is the last word

- **WHEN** the library default is switched on and no deck or card in the library sets a value
- **THEN** every card that carries a prompt queues in reverse

### Requirement: A deck carries a default template

The system SHALL let a deck hold a template that its cards inherit, SHALL let a card override it with
its own, SHALL let either be cleared, and SHALL show on the card which template is in force.

#### Scenario: A card inherits its deck's template

- **WHEN** a deck's template names `<<Word>>` and `<<Meaning>>` and a card in it has those two fields
- **THEN** the card renders through that template, and its editor says it inherits it

#### Scenario: A card's own template wins

- **WHEN** a card has its own template while its deck has a different one
- **THEN** the card renders through its own and the deck's other cards are unaffected

#### Scenario: Clearing a deck template leaves the cards readable

- **WHEN** a deck's template is cleared while its cards hold fields
- **THEN** each card renders its own markdown and the fields stay stored

### Requirement: Fields survive editing and export

The system SHALL keep a card's fields when its text, tags or deck change, SHALL keep them when it is
moved to the trash and restored, and SHALL search them as part of the card's content.

#### Scenario: A moved card keeps its fields

- **WHEN** a card with fields is moved to another deck with a different template
- **THEN** its fields are still there and the new template renders with them

#### Scenario: Search reaches a field value

- **WHEN** the user searches for a value that appears only in a field
- **THEN** the card is listed

### Requirement: A view is a saved filter with a name

The system SHALL let the user save the library's filter state as a named view, made of an optional deck
scope, a search text, a set of tags, a mode, a pinned-only flag and a sort order, SHALL list saved
views beside the decks, and SHALL apply every part of a view at once when it is selected.

#### Scenario: A saved view is applied whole

- **WHEN** a view saved as "Exam week" holds the tag `exam`, the mode active and the sort by due date,
  and the user selects it
- **THEN** the list shows active cards tagged `exam` from the whole library, ordered by due date

#### Scenario: A view survives a restart

- **WHEN** a view is created, the app is closed and reopened
- **THEN** the view is listed with the same name and the same filter

#### Scenario: A view can be renamed and deleted

- **WHEN** the user renames a view and later deletes it
- **THEN** the new name is stored and deleting it removes only the view, not the cards it matched

### Requirement: A card can be pinned

The system SHALL let any card be pinned, SHALL show it as pinned in the list, SHALL list only pinned
cards when a view asks for that, and SHALL keep the pin when the card is moved, archived, trashed and
restored.

#### Scenario: A pinned card is listed without a deck

- **WHEN** a view lists pinned cards and one card in a nested deck is pinned
- **THEN** that card is listed and its neighbours are not

#### Scenario: A pin survives the trash

- **WHEN** a pinned card is trashed and then restored
- **THEN** it is still pinned and its history is unchanged

### Requirement: The list can be ordered

The system SHALL order the list by last edited, created, first side, next due date, review count, or a
shuffle, in either direction where an order exists, and SHALL keep the chosen order when the list
refreshes.

#### Scenario: Ordering by due date puts the soonest first

- **WHEN** the user orders a list by due date
- **THEN** cards with the earliest due prompts come first and cards with no prompt come last

#### Scenario: A shuffle does not persist

- **WHEN** the user shuffles the list, then reloads another view and returns
- **THEN** the list is in the saved view's order again, because a shuffle is not a saved sort

#### Scenario: Ordering is not a schedule change

- **WHEN** any order is chosen
- **THEN** no prompt's due date or interval changes

### Requirement: Every card has a name

The system SHALL give each card a name, computed from its name template when it has one and otherwise
from its first side, SHALL keep names unique within a deck when it can do so by adding a suffix, SHALL
show the name wherever a card is listed, and SHALL update it when the text or fields it derives from
change.

#### Scenario: A card without a name template is named by its first side

- **WHEN** a card's first side is "What is mitosis?" and it has no name template
- **THEN** its name is "What is mitosis?" and the list shows that

#### Scenario: A name template wins

- **WHEN** a card's name template is `<<Word>> (verb)` and the field `Word` is "hablar"
- **THEN** its name is "hablar (verb)"

#### Scenario: Two cards may share a name in different decks

- **WHEN** two decks each hold a card named "Introduction"
- **THEN** both names stand and a reference to "Introduction" resolves inside the referencing deck first

### Requirement: A card can reference another

The system SHALL treat `[[Name]]` in a card's text as a reference, SHALL resolve it to the card with
that name preferring the referencing card's own deck and then its ancestors, SHALL render a resolved
reference as a control that opens the target card, and SHALL NOT alter a card's text when resolving.

#### Scenario: A reference opens the card it names

- **WHEN** a card contains `[[Krebs cycle]]` and a card with that name exists
- **THEN** the reference is drawn as a control, and using it opens the named card in the editor

#### Scenario: An unresolved reference stays text

- **WHEN** the reference names no card
- **THEN** it is drawn as ordinary text and reported as unresolved in the editor

#### Scenario: Resolution prefers the nearest deck

- **WHEN** two decks hold "Introduction" and a card in one of them references it
- **THEN** the reference opens the card in the referencing card's own deck

### Requirement: Every reference is listed on its target

The system SHALL list, on each card, the cards that reference it, SHALL update that list when either
side changes, and SHALL keep the list after a reference is renamed away by marking it unresolved.

#### Scenario: Backlinks appear on the target

- **WHEN** three cards reference "Krebs cycle"
- **THEN** that card's editor lists all three, and each entry opens the referencing card

#### Scenario: Renaming the target keeps the history of the link

- **WHEN** a referenced card is renamed so that no reference matches
- **THEN** the referencing cards still hold their `[[…]]` text and the target's backlink list is empty

### Requirement: Tags nest by path

The system SHALL treat a slash inside a tag as a path separator, SHALL list a tag's children under it,
and SHALL, when filtering by a tag, include the cards of its descendants.

#### Scenario: Filtering a parent includes its children

- **WHEN** cards are tagged `spanish/verbs` and `spanish/verbs/past` and the user filters by
  `spanish/verbs`
- **THEN** both cards are listed

#### Scenario: A tag path is one tag

- **WHEN** the user types `spanish/verbs` as a tag
- **THEN** it is stored as that one tag with two segments, not as two tags

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

### Requirement: A deck name is trimmed and cannot be blank

The library SHALL trim surrounding whitespace from a deck name when a deck is created or renamed,
and SHALL refuse a name that is empty after trimming, leaving the library unchanged. Uniqueness
under a parent SHALL be judged on the trimmed name.

#### Scenario: A padded name is stored trimmed

- **WHEN** a deck is created with the name ` Spanish `
- **THEN** the deck is stored as `Spanish`

#### Scenario: A blank name is refused

- **WHEN** a deck is created or renamed with the name `   `
- **THEN** the write is refused with an error naming the blank name and no deck changes

#### Scenario: Padding does not dodge the duplicate check

- **WHEN** a top-level deck `Spanish` exists and another top-level deck is created as ` Spanish`
- **THEN** the creation is refused as a duplicate

### Requirement: A new card goes to an open deck

A new card SHALL go to the selected deck when one is selected and it is not archived. Otherwise it
SHALL go to the first deck that is not archived. When there is no open deck, no card SHALL be made.

#### Scenario: The selected deck takes the card

- **WHEN** deck `Spanish` is selected and a new card is started
- **THEN** the card is made in `Spanish`

#### Scenario: An archived first deck is skipped

- **WHEN** no deck is selected, the first deck is archived and the second is not
- **THEN** a new card is made in the second deck

#### Scenario: No open deck makes nothing

- **WHEN** every deck is archived and a new card is started from the Inbox
- **THEN** no card is made and the editor does not open

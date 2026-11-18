# Spec Delta

## ADDED Requirements

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

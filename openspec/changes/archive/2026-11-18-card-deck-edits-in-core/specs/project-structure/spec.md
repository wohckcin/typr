# Spec Delta

## ADDED Requirements

### Requirement: Card and deck edits are decided in the backend

The rules that decide a card or deck write MUST live in the backend library: the deck a new card
goes to, the card selected after a trash, whether a deck name is acceptable, whether a card has
content, and what a save does to a stored draft. The desktop crate SHALL only map a gesture to a
backend call and turn its answer into screen state.

#### Scenario: The rules are tested without the toolkit

- **WHEN** the backend crate's tests are run
- **THEN** each of those rules has a test there that passes without any NSH crate

#### Scenario: Another front end gets the same answers

- **WHEN** a caller other than the desktop app creates a deck named `  ` or saves a text card as
  whitespace through the backend
- **THEN** the backend refuses it, as the desktop app does

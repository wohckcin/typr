## ADDED Requirements

### Requirement: A screen that changes state shows the change

A screen SHALL play at least one movement for the state change a reader is most likely to miss, or its
`DESIGN.md` row SHALL state why that change needs none. A row that plays SHALL name the tool, the token and what
reduced motion does.

#### Scenario: A panel opens

- **WHEN** the keymap panel or the palette opens
- **THEN** it enters from its trigger over the token its row names
- **AND** the row states the movement and its reduced-motion form

#### Scenario: A record arrives

- **WHEN** a review is recorded and the history is showing
- **THEN** the arriving record enters at the top of the list
- **AND** the rows already shown do not move

#### Scenario: A status word changes

- **WHEN** the editor's saved/unsaved word changes
- **THEN** the word rises, without a glyph swap

#### Scenario: A change that is not animated

- **WHEN** a screen's own row says no motion
- **THEN** the row gives a reason, and per-keystroke updates are one of them

### Requirement: Reduced motion removes the movement, not the change

Under reduced motion, a movement SHALL be skipped and the state change SHALL still be drawn, at the same layout
position and size the movement would have ended at.

#### Scenario: Every entrance

- **WHEN** reduced motion is on and a panel, a record or a word would enter
- **THEN** it appears placed and unscaled
- **AND** the frame after the change shows the new state

### Requirement: A motion token has a user

A token defined in `crates/typr/src/ui/motion.rs` SHALL be named by at least one screen or control, and SHALL be
deleted when nothing uses it.

#### Scenario: An unused token

- **WHEN** the tokens in `ui/motion.rs` are compared with the code that names them
- **THEN** every one has a user
- **AND** a planted unused token fails the check

## ADDED Requirements

### Requirement: A surface the base crate provides is built from it

A screen SHALL build a palette, a table, a tree or a list from the `nsh/base` element that provides it, and SHALL
NOT reimplement that element's row drawing, keyboard movement or accessibility state in the screen.

#### Scenario: The command palette

- **WHEN** the palette is opened on a surface
- **THEN** its rows are the base element's rows, one per item, virtualized and reachable with the arrow keys
- **AND** the screen decides only the items and their look

#### Scenario: The review history

- **WHEN** a card with reviews opens its history
- **THEN** each record is one row whose phase, grade, when and due are separate cells

#### Scenario: The deck tree

- **WHEN** the sidebar shows nested decks
- **THEN** the rows are a tree, folded by the tree's own expansion, moved through by the tree's own keys
- **AND** a parent's due count is still the count of its subtree

### Requirement: A hand-built element is a recorded decision

A component that draws what a `nsh/base` element already provides SHALL be named in `DESIGN.md` with the reason
it is drawn instead, and SHALL NOT be added without that line.

#### Scenario: The keycap

- **WHEN** `ui/status.rs`'s `Keycap` is compared with the base crate's elements
- **THEN** `DESIGN.md` states that `nsh-base`'s `keys.rs` binds keys and draws nothing, so the keycap is the app's

#### Scenario: A new list

- **WHEN** a screen needs a list of rows with a selection
- **THEN** it uses the base list or table, or the change that hand-builds it adds its `DESIGN.md` line

### Requirement: A label is not a status chip

A screen SHALL use a status chip for a status and text for a label, and SHALL NOT show a key chord as bare text
outside a keycap.

#### Scenario: The empty deck

- **WHEN** a deck holds no cards
- **THEN** the screen shows one sentence saying so and one button to add a card
- **AND** no chip stands in for the label the button already carries

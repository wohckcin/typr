## MODIFIED Requirements

### Requirement: A hand-built element is a recorded decision

A component that draws what a `nsh/base` element already provides SHALL be named in `DESIGN.md` with the reason
it is drawn instead, and SHALL NOT be added without that line. An element `DESIGN.md` names as a `nsh/base`
element SHALL be built from that element.

#### Scenario: The keycap

- **WHEN** `ui/status.rs`'s `Keycap` is compared with the base crate's elements
- **THEN** `DESIGN.md` states that `nsh-base`'s `keys.rs` binds keys and draws nothing, so the keycap is the app's

#### Scenario: A new list

- **WHEN** a screen needs a list of rows with a selection
- **THEN** it uses the base list or table, or the change that hand-builds it adds its `DESIGN.md` line

#### Scenario: The result line

- **WHEN** the diagnostics screen states what an import or an export came to
- **THEN** the line is a `nsh/base` toast in a toast stack, as `DESIGN.md` says

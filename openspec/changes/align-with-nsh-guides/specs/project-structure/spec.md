# Spec Delta

## ADDED Requirements

### Requirement: A public struct widens without breaking its callers

A public struct with public fields SHALL be `#[non_exhaustive]`, so a caller outside the crate cannot build
it with a struct literal and a later field is an addition. It SHALL offer a construction that sets only what
the caller means to set: `Default` plus field assignment, or a constructor. A struct that owns reactive
state MAY omit `Default` when its constructor is its only way to exist and the type documents why.

#### Scenario: A caller outside the crate still builds it

- **WHEN** a caller in another crate builds a public struct with public fields
- **THEN** it compiles through the type's `Default` and assignment or through its constructor, and no struct
  literal or functional update is needed

#### Scenario: A field is added

- **WHEN** a field is added to a public struct with public fields
- **THEN** no caller outside the crate needs a change

#### Scenario: Reactive state names its constructor

- **WHEN** a struct's fields are signals its caller's owner creates
- **THEN** it is built only through its constructor, and it documents why it carries no `Default`

### Requirement: A component's events carry one named object

Each event a component reports SHALL carry one object whose fields name the values it reports, and its
name SHALL say what the user did. A lone value or a data type SHALL NOT be the payload itself.

#### Scenario: Payloads name their values

- **WHEN** the editor reports that the card was saved
- **THEN** the handler receives an object whose field names the text

#### Scenario: Handlers read as acts

- **WHEN** a component's event names are listed
- **THEN** each names the act the user performed rather than the state it left behind

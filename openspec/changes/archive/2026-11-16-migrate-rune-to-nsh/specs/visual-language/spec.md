## REMOVED Requirements

### Requirement: The appearance is checked against a baseline

**Reason**: the port no longer captures screenshots, so a committed baseline image has nothing to be
compared with.

**Migration**: `tests/appearance.rs` renders each screen headlessly in both appearances and reads the
resolved styles, which is what the requirement below asks for.

## MODIFIED Requirements

### Requirement: Every screen draws with one token set

The system SHALL draw spacing, type sizes, radii, elevation, motion and the focus ring from the app's
theme and its `ui` layer, SHALL NOT use a numeric literal or a raw colour for those inside a screen, and
SHALL keep the same token set in light and dark themes.

#### Scenario: A screen uses the scale

- **WHEN** any screen's source is searched for a spacing, font-size, radius or colour literal
- **THEN** none is found, because each comes from the theme or a `ui` component; the only exceptions are
  documented physical boundaries such as a hairline

#### Scenario: Both themes share the tokens

- **WHEN** the appearance is changed between light and dark
- **THEN** every dimension stays identical and only the colours change

#### Scenario: Screens use only the ui layer

- **WHEN** a screen module's imports are inspected
- **THEN** it builds controls from the app's `ui` module and does not construct base controls directly

## ADDED Requirements

### Requirement: The appearance is checked by the styles it resolves to

The system SHALL exercise every screen headlessly in the light and the dark appearance, SHALL assert
that the same text resolves to the same size and weight in both, SHALL assert that its colour is the
theme's colour for that role and appearance and so changes between them, and SHALL fail when a screen
has no such check rather than skipping it.

#### Scenario: Dimensions do not move with the appearance

- **WHEN** a screen is rendered in the light appearance and then in the dark one
- **THEN** the same text resolves to the same font size and weight, and only its colour differs

#### Scenario: A screen's colour comes from the theme

- **WHEN** a screen's text is read in each appearance
- **THEN** its colour is the theme colour for that role and appearance, not a literal, so changing the
  theme changes it

#### Scenario: A screen without the check fails

- **WHEN** a screen module has no headless check in both appearances
- **THEN** the appearance check fails and names the screen, rather than skipping it

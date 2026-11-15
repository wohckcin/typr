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

### Requirement: The appearance is checked against a baseline

The system SHALL capture each screen on every supported platform, in light and dark, SHALL compare
each capture with a committed baseline image for that platform, SHALL fail on a difference larger than
the stated tolerance, and SHALL fail when a screen or a baseline is missing.

#### Scenario: An unchanged screen matches

- **WHEN** a screen is captured and nothing changed
- **THEN** its baseline comparison passes in both appearances

#### Scenario: A changed screen fails

- **WHEN** a spacing token changes
- **THEN** at least one baseline comparison fails and names the screen and appearance

#### Scenario: A new scene needs a baseline

- **WHEN** a screen capture is added without a baseline image
- **THEN** the comparison fails rather than skipping it

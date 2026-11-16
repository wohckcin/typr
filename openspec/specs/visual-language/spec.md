# visual-language Specification

## Purpose

Make every screen read as one application: one module of spacing, type, radius and surface tokens, cards
that look like objects, a review canvas that holds one thing, a sidebar that groups what the user has,
and an empty state wherever a screen has nothing to show — with the result checked against recorded
scene pictures rather than described.

## Requirements

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

### Requirement: A card is one object on the page

The system SHALL render a card as a single surface with a radius, a border and one step of elevation,
SHALL place everything the card shows inside that surface, SHALL show its metadata as secondary text, and
SHALL show a hover and a focus state that do not move it. What a card shows is `card-grid`'s to decide.

#### Scenario: A tile reads as one card

- **WHEN** the library draws a card
- **THEN** its text, its badges and its chips share one border, and hovering changes only colour and
  shadow

#### Scenario: Focus is visible without motion

- **WHEN** a card is focused by the keyboard
- **THEN** a focus ring is drawn inside its bounds and no other element shifts

### Requirement: The review surface holds one thing

The system SHALL draw a review on a dimmed canvas with the card centred, SHALL separate the prompt from
the revealed answer by spacing and type weight rather than by a rule, SHALL keep the answer area the same
size before and after the reveal, and SHALL show the keyboard hints beneath the card.

#### Scenario: Revealing does not move the card

- **WHEN** the answer is revealed
- **THEN** the card's outer bounds are unchanged and the answer appears inside them

#### Scenario: The hints sit under the card

- **WHEN** a review is open
- **THEN** the bindings are drawn below the card, secondary, and never inside it

### Requirement: The sidebar groups what the user has

The system SHALL group the sidebar into Decks, Tags and Views under headings, SHALL show a count beside
each group and beside each deck, and SHALL keep the grouping while filtering.

#### Scenario: Groups are labelled and counted

- **WHEN** the library holds three decks, two tags and one view
- **THEN** the sidebar shows those three headings, each with its count

#### Scenario: An empty group is still named

- **WHEN** no view exists
- **THEN** the Views heading is drawn with a count of zero and an action to make one

### Requirement: Every empty screen says what to do

The system SHALL draw an empty state in place of an empty list or queue, SHALL name what is missing, and
SHALL offer the action that fills it.

#### Scenario: An empty library offers the first card

- **WHEN** the library holds no card
- **THEN** the list area shows an empty state whose action creates a card

#### Scenario: An empty queue offers a cram session

- **WHEN** nothing is due
- **THEN** the review entry point says so and offers a cram session instead

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

### Requirement: A control takes one size

A styled control that has a size SHALL take it as one value that sets its height, horizontal padding, text
size, icon size and radius together, and no call site SHALL set those one by one. A control whose geometry cannot follow the user's text size
SHALL be documented as a physical boundary with its reason.

#### Scenario: One value sets the frame

- **WHEN** the same control is drawn at its small and its default size
- **THEN** height, horizontal padding, text size and radius each change together, and the call site chose
  only the size

#### Scenario: Larger text does not clip

- **WHEN** the user's text size is raised
- **THEN** a control's label grows and its frame grows with it, because its height is a minimum

### Requirement: Pressable controls answer hover and press

Every pressable control SHALL show a hover feedback where a pointer exists and a press feedback on
activation, drawn from the app's motion tokens and absent while the control is disabled. Hover SHALL NOT
change the control's size, padding or layout, and press feedback SHALL be stronger than hover.

#### Scenario: Hover is a wash

- **WHEN** the pointer rests on an enabled button
- **THEN** its surface changes without moving its label or its neighbours

#### Scenario: Press is deeper than hover

- **WHEN** the button is held down
- **THEN** the press feedback is stronger than the hover, and the control returns to its rest state once
  released

#### Scenario: Disabled ignores the pointer

- **WHEN** a disabled button is hovered and pressed
- **THEN** neither state appears

#### Scenario: Reduced motion keeps the states

- **WHEN** the system asks for reduced motion
- **THEN** hover and press appear instantly and remain distinguishable

### Requirement: Motion matches the written inventory

Each screen's motion SHALL match the inventory kept in the app's design document. Every inventory line
SHALL either describe motion the app performs, checked in the running app, or state that no motion is used
and why. Motion SHALL be drawn from the app's motion tokens and SHALL stop when reduced motion is on.

#### Scenario: The inventory and the app agree

- **WHEN** a screen's inventory lines are compared with the running app
- **THEN** each line either plays as written or says no motion with a reason

#### Scenario: The current row changes

- **WHEN** the current sidebar row changes
- **THEN** the row's own fill changes in place, and the inventory says why no fill slides: the sidebar
  keeps no row geometry, so a sliding fill would need a measurement layer for one transition

#### Scenario: Reduced motion stops it

- **WHEN** the system asks for reduced motion
- **THEN** the moving selection and every transition appear instantly, and every state stays readable

### Requirement: Popups share one surface

Menus, popovers and dialogs SHALL draw on the theme's popover surface, with its background, border, radius
and elevation, so the popup family cannot drift apart.

#### Scenario: One popup surface

- **WHEN** a menu and a popover are opened
- **THEN** both use the theme's popover background, border and radius

### Requirement: A command that needs more input says so

A command whose activation opens a dialog, a sheet, a window or a file chooser SHALL end its label with a
single ellipsis character. A command that acts immediately SHALL NOT.

#### Scenario: Opening commands take an ellipsis

- **WHEN** the editor draws its attach-image, diagram and clear-diagram commands
- **THEN** each label ends with an ellipsis character

#### Scenario: Immediate commands do not

- **WHEN** the editor draws its Save command
- **THEN** its label carries no ellipsis

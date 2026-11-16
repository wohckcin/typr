# Spec Delta

## ADDED Requirements

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

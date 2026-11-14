# Spec Delta — card-editor

## ADDED Requirements

### Requirement: A card is either text or a diagram

The system SHALL let a card be created as a diagram, made of one image, an optional caption and a set
of rectangles, and SHALL keep a text card's editable markdown untouched. Switching an empty text card
to a diagram and back SHALL be permitted; a card holding either content SHALL NOT be switched without
clearing it first.

#### Scenario: A diagram card is not a markdown card

- **WHEN** a card is created as a diagram with an image and two groups
- **THEN** its editor shows the image and the rectangles instead of a markdown source pane, and the
  card does not report sides

#### Scenario: A text card keeps its text

- **WHEN** a card with markdown content is opened
- **THEN** it is still edited as markdown and the diagram controls are not offered

### Requirement: Rectangles are drawn on the image by dragging

The system SHALL add a rectangle when the user drags across the image, SHALL store each rectangle as
fractions of the image's width and height so it holds at any display size, and SHALL refuse a
rectangle narrower or shorter than one percent of the image.

#### Scenario: A drag adds a rectangle

- **WHEN** the user drags from one point of the image to another
- **THEN** a rectangle spanning those points is stored, and the same rectangle appears at the same
  place when the image is displayed larger

#### Scenario: A click is not a rectangle

- **WHEN** the user presses and releases without moving
- **THEN** no rectangle is added

### Requirement: Rectangles are grouped, and a group is one review

The system SHALL let each rectangle belong to a numbered group, SHALL let several rectangles be merged
into one group, SHALL let a group be removed, and SHALL show which group each rectangle belongs to
while editing.

#### Scenario: Merging two rectangles makes one review

- **WHEN** the user merges two rectangles into group 1
- **THEN** the diagram holds one group, and one review covers both rectangles together

#### Scenario: Removing a group leaves the history alone

- **WHEN** a group that has been reviewed is removed from the card
- **THEN** the card no longer queues that group and its records remain readable

### Requirement: The image is copied into the library

The system SHALL copy a chosen image into the library's attachments folder, SHALL reference it by file
name only, and SHALL report a chosen file it cannot copy without changing the card.

#### Scenario: A chosen image survives its source being deleted

- **WHEN** an image is chosen, saved, and its original file is then deleted
- **THEN** the card still displays the image after a restart

#### Scenario: An unreadable image is reported

- **WHEN** the chosen file cannot be copied
- **THEN** the card keeps its previous image or none, and the failure is shown on screen

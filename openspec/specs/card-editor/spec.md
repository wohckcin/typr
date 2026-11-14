# card-editor Specification

## Purpose
Defines how a card is written: its text as markdown, the separator that splits it into the sides shown during study, and what the user sees and loses while editing.

## Requirements

### Requirement: A card's text is markdown

The system SHALL store each card's content as a markdown document and SHALL render headings, emphasis, lists, tables, task lists, fenced code, blockquotes, links and local images in the preview and during study.

#### Scenario: Markdown renders in the preview

- **WHEN** the card's text is `# Title` followed by `- one` on the next line
- **THEN** the preview shows a heading and a bulleted list, not the literal source

### Requirement: A line whose only content is three hyphens separates sides

The system SHALL treat a line as a side separator when, ignoring surrounding spaces and a trailing carriage return, its content is exactly `---`, and SHALL support two or more sides in one card. A line inside a fenced code block SHALL never separate. A separator SHALL NOT be rendered as content, and a side that is empty after trimming SHALL be dropped. `***` and `___` remain available to the user as thematic breaks.

#### Scenario: Storing a two-sided card

- **WHEN** the user saves the text `Question` then `---` then `Answer`
- **THEN** the card has two sides and its first side is `Question`

#### Scenario: Padding and Windows line endings still separate

- **WHEN** the text contains the line `  ---  ` followed by a carriage return
- **THEN** a side is created

#### Scenario: Hyphens inside a side are not separators

- **WHEN** the text contains `---` on a line that also contains other characters, or contains `***`, or contains `---` inside a fenced code block
- **THEN** no side is created and the text renders as part of the current side

#### Scenario: An empty side is ignored

- **WHEN** the user saves `Question`, `---`, and nothing else
- **THEN** the card has one side and is treated as a note

### Requirement: Local images resolve against an attachments directory

The system SHALL resolve a relative image path in card markdown against an attachments directory inside the app's data directory, SHALL make that image appear in the preview and during study, SHALL NOT resolve a path that escapes the attachments directory, and SHALL draw a visible placeholder for a reference whose file is missing. The system SHALL provide an action that copies a chosen image file into the attachments directory and inserts its reference at the cursor.

#### Scenario: A copied image renders in study

- **WHEN** the user attaches an image and saves the card
- **THEN** the image appears in the preview and on the card's side during study

#### Scenario: A path escaping the directory is refused

- **WHEN** the card's markdown references an image path that escapes the attachments directory
- **THEN** the image is not loaded and a placeholder is drawn

### Requirement: Editing a card never resets its schedule

The system SHALL keep a card's scheduling state, history and prompt key unchanged when its text changes, including when the edit changes the number of sides or turns a two-sided card into a note and back. A card that becomes a note SHALL stop appearing in study queues and SHALL return to them with its due date intact when it becomes two-sided again.

#### Scenario: Adding a third side keeps the due date

- **WHEN** the user adds a side to a card that is due in five days
- **THEN** the card still has one prompt and its due date is unchanged

#### Scenario: Turning a card into a note pauses it

- **WHEN** the user removes the separator so the card has one side
- **THEN** the card leaves the study queues and its due date is preserved

### Requirement: The editor shows source and preview together

The system SHALL present the card's markdown source in an editable text area and SHALL show a rendered preview of the same content, kept consistent as the user types. The preview SHALL reflect side boundaries so the user can see each side as it will be studied.

#### Scenario: Preview follows typing

- **WHEN** the user types `**bold**` into the source
- **THEN** the preview shows the word in bold without the user switching modes

### Requirement: Saves are explicit and interrupted edits are recoverable

The system SHALL require a deliberate save action to commit an edit, SHALL show when a card has unsaved changes, SHALL warn before abandoning an editor with unsaved changes, and SHALL preserve the in-progress text across a restart until it is saved or discarded.

#### Scenario: Warning on abandoning unsaved work

- **WHEN** the user navigates away from a card with unsaved changes
- **THEN** the app asks whether to save, discard, or stay

#### Scenario: Unsaved text survives a restart

- **WHEN** the app exits with unsaved changes in the editor
- **THEN** the next launch restores that text in the editor, marked as unsaved

### Requirement: A card with no content is not saved

The system SHALL refuse to save a card whose sides are all empty or whitespace, and SHALL report why the save was refused.

#### Scenario: Empty card is refused

- **WHEN** the user attempts to save a card whose text is only whitespace
- **THEN** no card is created and the editor reports that the card is empty

### Requirement: Hidden text is written inline and previewed masked

The editor SHALL accept hidden text in the card's markdown as typed, SHALL preview it masked in the
rendered pane, and SHALL show which group a mask belongs to. A card holding hidden text SHALL ignore
the three-hyphen separator, so the preview shows one face rather than two sides.

#### Scenario: The preview shows the mask, not the answer

- **WHEN** the user types `Our lungs pull {{2::oxygen}} from the {{1::air}}.`
- **THEN** the preview shows the sentence with both spans masked and reports two prompts

#### Scenario: Unnumbered spans are one prompt

- **WHEN** the user types `The hormone {{insulin}} is produced by the {{pancreas}}.`
- **THEN** the preview masks both spans and reports one prompt

#### Scenario: A separator stops splitting a hidden-text card

- **WHEN** a card holds hidden text and a line of three hyphens
- **THEN** the preview reports one face and the separator is shown as ordinary content

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

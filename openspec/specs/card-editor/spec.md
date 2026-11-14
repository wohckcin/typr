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

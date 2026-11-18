## ADDED Requirements

### Requirement: A transfer result is stated, then leaves

The diagnostics screen SHALL state what an import or an export came to as a notification, and the notification
SHALL leave on its own after a reading timeout, without a press.

#### Scenario: An export result leaves

- **WHEN** an export finishes and its result is shown
- **THEN** the result is on screen
- **AND** after the timeout it is gone, with no press

#### Scenario: A later result replaces nothing by accident

- **WHEN** a second import finishes after the first result has left
- **THEN** only the second result is on screen

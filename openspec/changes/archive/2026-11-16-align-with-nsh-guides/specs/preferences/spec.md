# Spec Delta

## ADDED Requirements

### Requirement: A settings field carries a visible label

A setting that is edited through a text field SHALL name itself with visible text beside the field. The
field's placeholder SHALL NOT be its only name, and the label SHALL stay visible while the field holds a
value.

#### Scenario: The label survives a value

- **WHEN** the daily new-card limit holds a number
- **THEN** the text naming the setting is still visible beside the field

#### Scenario: The setting is named without its placeholder

- **WHEN** the settings screen is read with the field's placeholder hidden
- **THEN** the purpose of the field is still stated by the label

## ADDED Requirements

### Requirement: Settings audio row
The system SHALL show the Music toggle on the audio slider's row, right column.

#### Scenario: Music
- **WHEN** the settings are read
- **THEN** row 3 is Music, a toggle, on row 2 column 1

### Requirement: Controls bindings
The system SHALL provide 22 control bindings, including the eight lower rows Skills, Journal, Talk, Use Power, Power Bar, Previous Power, Next Power and Power Tree.

#### Scenario: Lower rows
- **WHEN** the controls are read
- **THEN** the list has 22 rows and Skills is K, Journal J and Talk T
- **WHEN** Power Tree is read
- **THEN** its default key is empty

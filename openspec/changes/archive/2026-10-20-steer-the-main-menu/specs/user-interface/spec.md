## ADDED Requirements

### Requirement: Main-menu selection
The system SHALL keep a row selection per page and clamp a move to the row range without wrapping.

#### Scenario: Ends
- **WHEN** the title page starts at row 0 and moves up
- **THEN** it stays at row 0
- **WHEN** it moves down past the last row
- **THEN** it stops at the last row

### Requirement: Main-menu activation
The system SHALL turn the selected enabled row into its action and return `None` for a disabled row.

#### Scenario: Continue and novice
- **WHEN** CONTINUE is selected without a save
- **THEN** activation returns `None`
- **WHEN** it is selected with a save
- **THEN** activation returns Continue
- **WHEN** NOVICE is disabled
- **THEN** activation returns `None`

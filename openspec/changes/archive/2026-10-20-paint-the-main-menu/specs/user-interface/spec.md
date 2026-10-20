## ADDED Requirements

### Requirement: Main-menu paint
The system SHALL start rows at `trunc(H·origin) + line_height`, advance by `trunc(70·row_h)`, centre on the text width, grey disabled rows and bracket styled labels.

#### Scenario: Title paint
- **WHEN** the title paints at 1600×1080 with origin 0.38, row_h 1 and line height 49
- **THEN** the first row is at y 459 and the next at 529
- **WHEN** CONTINUE is disabled and style 1
- **THEN** its colour is (128,128,128,128) and its text is [CONTINUE]

### Requirement: Selected row last
The system SHALL draw the selected row last and nudge it by unit/3 when it is the final row.

#### Scenario: Last row
- **WHEN** the last of three rows is selected with unit 6
- **THEN** it is drawn last at its row y plus 2

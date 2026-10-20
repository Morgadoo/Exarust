## ADDED Requirements

### Requirement: Main-menu row advance
The system SHALL advance rows by `trunc(70·row_h)` and start the first row at `trunc(height·origin)`.

#### Scenario: Unit row height
- **WHEN** row_h is 1.0
- **THEN** the advance is 70
- **WHEN** the title height is 1080 and the origin 0.38
- **THEN** the first row starts at 410 and the second at 480

### Requirement: Main-menu row hit
The system SHALL hit row `i` when the point lies in `[top + i·advance, top + i·advance + band)`.

#### Scenario: Hit and miss
- **WHEN** the point is 456 with a 70 band
- **THEN** it hits row 0
- **WHEN** the point is 409 or 900
- **THEN** it hits no row

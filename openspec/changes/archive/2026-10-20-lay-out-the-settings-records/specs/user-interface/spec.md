## ADDED Requirements

### Requirement: Settings record anchors
The system SHALL resolve a record's y anchor by adding its delta to the previous row's grid y, negated when style bit 0x80000 is set, and its x anchor from the previous row's right edge.

#### Scenario: Anchors
- **WHEN** the y code is 0x8006
- **THEN** the delta is 9 grid units
- **WHEN** the x code is 0x8002
- **THEN** the gap after the previous row is 72 px
- **WHEN** a record's size is zero
- **THEN** the class skin size is used

### Requirement: Settings records
The system SHALL provide the 9 tab/button, 27 settings and 22 binding records and lay them out at unit 4 on a 434×536 form with a 408-px settings list and a 416-px controls view.

#### Scenario: Unit four
- **WHEN** the form is laid out at unit 4
- **THEN** the first tab is (12,20,192,32), the first settings row is at y 64 and the first binding at y 64
- **WHEN** the unit is 6
- **THEN** the form is 651×804 and the first tab is (18,30,288,48)

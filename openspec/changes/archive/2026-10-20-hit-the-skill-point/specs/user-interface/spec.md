## ADDED Requirements

### Requirement: Skill point guards
The hit SHALL return 0 when state bit 1 is set and the page is below 2, and the base result when the form does not own the point or the base is not −1 and the guard refuses. On page 2 a point in the bottom band SHALL return `0x10000`. A state byte of 4 SHALL return the base result.

#### Scenario: Early returns and the band
- **WHEN** state bit 1 is set and the page is 1
- **THEN** the result is 0
- **WHEN** the form does not own the point
- **THEN** the base result is returned
- **WHEN** the page is 2, the size is 200 by 100, the unit 4 and the point 100, 50
- **THEN** the result is `0x10000`
- **WHEN** the state byte is 4
- **THEN** the base result is returned

### Requirement: Skill point nodes
A high selection SHALL give 1 for a horizontal offset between 13 and 4 units to the left and 2 for the same to the right, within 4 units vertically, else −1. A low selection SHALL give the rounded slot from 4.5 plus the offset over ten units when it is 1 through 8, else −1. Without selections the fallback SHALL run.

#### Scenario: High, low and fallback
- **WHEN** the high node is at 100, 50, the unit 4 and the point 80, 50
- **THEN** the result is 1
- **WHEN** the point is 120, 50
- **THEN** the result is 2
- **WHEN** the low node is at 100, 40 and the point 120, 40
- **THEN** the slot is 5
- **WHEN** no selection exists
- **THEN** the fallback result is returned

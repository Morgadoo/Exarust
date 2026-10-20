## ADDED Requirements

### Requirement: Skill node position
The position SHALL be the layout point at the row in bits 8 through 15 and the column in the low 8 bits of a positive index, rounded to the nearest even integer. A non-positive index SHALL take the base position when the scaled flag is set, otherwise half the width and half the height, truncated toward zero.

#### Scenario: Placed, centred and forwarded
- **WHEN** the index is 0x0205 and the layout returns 12.5 and 34.5
- **THEN** the row is 2, the column 5 and the position 12, 34
- **WHEN** the index is 0, the bounds are 10, 110, 20 and 80 and the flag is clear
- **THEN** the position is 50, 30
- **WHEN** the index is −1 and the flag is set
- **THEN** the base position is returned

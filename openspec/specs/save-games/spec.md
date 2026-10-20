# save-games Specification

## Purpose
The save game package: the settings chunk, its tags and versions, and the easing curves the save screens use.

## Requirements

### Requirement: Save easing curve
Easing SHALL return `(x × 2)² × 0.5` below 0.5 and `((2x − 3)(2x − 1) − 1) × −0.5` otherwise.

#### Scenario: The curve
- **WHEN** the input is 0.25
- **THEN** the result is 0.125
- **WHEN** the input is 0.5
- **THEN** the result is 0.5
- **WHEN** the input is 0.75
- **THEN** the result is 0.875

### Requirement: Save settings chunk
Writing SHALL use the header `0xECADCA01`, a small or large tag and two dwords. Reading SHALL accept only that header, set the old flag when 1,000,000 is above the tag float, and store the dwords only on acceptance.

#### Scenario: Write and read
- **WHEN** the large tag is written with 3 and 4
- **THEN** the header is `0xECADCA01`, the tag is large and the dwords are 3 and 4
- **WHEN** the header matches and the tag float is 999,999
- **THEN** the read is accepted, the old flag is set and both dwords are stored
- **WHEN** the tag float is 1,000,000
- **THEN** the old flag is clear
- **WHEN** the header differs
- **THEN** the read is rejected and no dword is stored

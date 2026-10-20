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

### Requirement: Save version gate
Conversion SHALL be false unless the mode is 4 and the pending count is zero. A legacy entry SHALL convert. Otherwise a present and read record SHALL convert when its tag is not `0xDBAC0D06`, and the same tag SHALL not.

#### Scenario: Legacy, current and refused
- **WHEN** the mode is 4, pending is 0 and the legacy entry is present
- **THEN** conversion is needed
- **WHEN** the record is read with tag `0xDBAC0D06`
- **THEN** conversion is not needed
- **WHEN** the tag is 0
- **THEN** conversion is needed
- **WHEN** the mode is 3 or pending is positive
- **THEN** conversion is not needed

### Requirement: Save record table
Writing SHALL write the count first, then each record's kind and two qwords.

#### Scenario: Two records
- **WHEN** two records are written
- **THEN** the count is 2
- **AND** each record keeps its kind, first qword and second qword in order

### Requirement: Save number format
Formatting SHALL give `"000"` for zero or a negative value. Otherwise it SHALL write the decimal digits and pad on the left with zeros to three characters, leaving longer values whole.

#### Scenario: Zero, short and long
- **WHEN** the value is 0
- **THEN** the text is 000
- **WHEN** the value is 5
- **THEN** the text is 005
- **WHEN** the value is 12
- **THEN** the text is 012
- **WHEN** the value is 1234
- **THEN** the text is 1234
- **WHEN** the value is −7
- **THEN** the text is 000

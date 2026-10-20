## ADDED Requirements

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

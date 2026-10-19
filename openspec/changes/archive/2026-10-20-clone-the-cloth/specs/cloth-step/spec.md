## ADDED Requirements

### Requirement: Cloth clone
The clone SHALL create one new cloth and copy fields 0x20, 0x140, 0x144 and 0x148 unchanged.

#### Scenario: The copy
- **WHEN** the fields are 0x1122, 0x33, 0x44 and 0x55
- **THEN** the new cloth is created and holds 0x1122, 0x33, 0x44 and 0x55

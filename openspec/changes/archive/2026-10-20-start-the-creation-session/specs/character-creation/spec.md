## ADDED Requirements

### Requirement: Creation table selection
Selecting the table SHALL store the source index less one and apply that entry.

#### Scenario: The selection
- **WHEN** the source index is 4
- **THEN** the stored index is 3 and the entry is applied

### Requirement: Creation session start
Starting the session SHALL store the value, clear the stored byte and schedule.

#### Scenario: The start
- **WHEN** the session starts
- **THEN** the value is stored, the byte clears and the schedule runs

### Requirement: Creation menu dispatch
The menu dispatch SHALL take the alternate call when the global byte is set and the primary call otherwise.

#### Scenario: Alternate and primary
- **WHEN** the byte is set
- **THEN** the alternate call runs
- **WHEN** the byte is clear
- **THEN** the primary call runs

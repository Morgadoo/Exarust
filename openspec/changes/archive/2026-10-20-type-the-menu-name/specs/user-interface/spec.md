## ADDED Requirements

### Requirement: Main-menu name entry
A key without bit 0x2 SHALL go to the base handler and leave the buffer. Otherwise the ten bytes SHALL shift left by one and the character SHALL be appended, with A–Z lowered by 0x20. A buffer of "barechoons" SHALL run the submit routine.

#### Scenario: The shift
- **WHEN** the buffer is "?barechoon" and the character is "s"
- **THEN** the buffer becomes "barechoons" and the submit routine runs
- **WHEN** the character is "A"
- **THEN** the last byte becomes "a"

#### Scenario: The base path
- **WHEN** the key has no bit 0x2
- **THEN** the base handler runs and the buffer is unchanged

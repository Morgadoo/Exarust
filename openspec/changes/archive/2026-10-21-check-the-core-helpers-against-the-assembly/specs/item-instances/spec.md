# Spec Delta

## ADDED Requirements

### Requirement: Factory grade gate
The grade routine SHALL copy and grade only a record whose name section's flag word has bit 0 and whose model starts with `@`, return the id unchanged for another record, and return 0 for a record it cannot read or without a name section.

#### Scenario: Ungraded
- **WHEN** a factory weapon's name flag word is 0
- **THEN** its own id is returned and nothing is copied

### Requirement: Picked copy condition
A picked copy's item body, checked before its weapon section, SHALL take the row's damage, dirt and spreads in its appearance block and a palette byte of its low two bits, the magnitude of 31 × (row wealth − those bits) rounded to nearest even in bits 2 and up, and the top bit for a negative difference.

#### Scenario: The palette byte
- **WHEN** a row of wealth 0 is applied to an apparel copy whose palette byte is 1
- **THEN** the byte becomes 0xFD

### Requirement: World-load refresh walk
The world-load walk SHALL refresh an item-body record whose body byte 0x1A8 is not zero, and every weapon record.

#### Scenario: A zero byte
- **WHEN** an apparel record's body byte 0x1A8 is 0
- **THEN** it is not refreshed

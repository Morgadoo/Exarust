## ADDED Requirements

### Requirement: Combat forget
The forget SHALL remove the first entry equal to the target by moving the last entry over it and decrementing the count, or leave the list unchanged when absent. It SHALL zero the slots at 0x778 and 0x770 and the body's hit and contact pointers when they equal the target.

#### Scenario: The removal
- **WHEN** the entries are 1, 2, 3 and the target is 2
- **THEN** the entries become 1, 3 and the count 2
- **WHEN** the target is the last entry
- **THEN** it is dropped
- **WHEN** the target is absent
- **THEN** the list is unchanged

#### Scenario: The clears
- **WHEN** a stored slot or the body's hit or contact pointer equals the target
- **THEN** it becomes zero

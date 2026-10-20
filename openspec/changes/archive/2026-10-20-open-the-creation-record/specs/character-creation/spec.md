## ADDED Requirements

### Requirement: Creation record reset
Resetting SHALL set the five words at `0x64` through `0x84` and the tail at `0xf0` to zero, then run the reset with size 8.

#### Scenario: The reset
- **WHEN** the record resets
- **THEN** the five words and the tail are zero
- **AND** the reset size is 8

### Requirement: Creation slot defaults
The slot table SHALL hold three eight-byte entries with the low word `0x61A8`, bytes 1, 2, 3 and 4, and byte 6 equal to 8, 5 and 6.

#### Scenario: The slots
- **WHEN** the slots are filled
- **THEN** the first is `0xA8`, `0x61`, 1, 2, 3, 4, 8, 0
- **AND** the second is `0xA8`, `0x61`, 1, 2, 3, 4, 5, 0
- **AND** the third is `0xA8`, `0x61`, 1, 2, 3, 4, 6, 0

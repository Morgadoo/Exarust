## ADDED Requirements

### Requirement: Class navigation
Mask 0x100 SHALL confirm a positive selection with result 7 and the class form, and return 0 for no selection. Masks 0x200 and 0x400 SHALL call the forward helper. Direction 0x8000 SHALL move down to the first available class at or after the selection up to 3; direction 0x4000 SHALL move up from selection − 2, or to 1 when at most 2. The move SHALL run only when the target differs.

#### Scenario: The confirm
- **WHEN** the mask is 0x100 and the selection is 3
- **THEN** the result is 7 and the class form shows
- **WHEN** the selection is 0
- **THEN** the result is 0

#### Scenario: The bit scans
- **WHEN** the available global is 0b1010, so classes 0, 2 and 4 exist
- **THEN** down from 1 moves to 3
- **AND** up from 4 moves to 3
- **AND** up from 2 moves to 1
- **AND** down from 4 does not move

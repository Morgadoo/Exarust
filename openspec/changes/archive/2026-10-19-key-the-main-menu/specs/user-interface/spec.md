## ADDED Requirements

### Requirement: Main-menu key
Mask 0x100 SHALL confirm a positive selection with the sound and suppress a zero one. Mask 0x200 SHALL call forward and 0x400 back. Direction 0x8000 SHALL move to max(selection + 1, 1) while below the limit; direction 0x4000 SHALL move to selection − 1 while above 1. The result SHALL be the helper's result or 0.

#### Scenario: The confirm
- **WHEN** the mask is 0x100 and the selection is 5
- **THEN** the confirm runs with the selection, a sound plays and the helper result returns
- **WHEN** the selection is 0
- **THEN** nothing runs and the result is 0

#### Scenario: The directions
- **WHEN** the mask is 0x8001, the selection is 2 and the limit is 4
- **THEN** the selection moves to 3
- **WHEN** the selection is 4
- **THEN** it does not move
- **WHEN** the selection is 0
- **THEN** it moves to 1
- **WHEN** the mask is 0x4001 and the selection is 3
- **THEN** it moves to 2

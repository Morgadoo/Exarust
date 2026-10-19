## ADDED Requirements

### Requirement: Main-menu confirm
A selection of zero or less SHALL copy field 0x70 to 0x204 and return 0x40. A positive selection with direction 0x1000 SHALL store itself at 0x70, copy it, play a sound when ready and return 7; any other direction SHALL return 0x40 without copying.

#### Scenario: The confirmation
- **WHEN** the selection is 3 and the direction is 0x1000
- **THEN** 3 is stored, copied to 0x204, the sound plays and the result is 7
- **WHEN** the selection is 3 and the direction is 0x2000
- **THEN** nothing is copied and the result is 0x40
- **WHEN** the selection is 0
- **THEN** 0x70 copies to 0x204 and the result is 0x40

### Requirement: Main-menu select
The select SHALL store the index and return 3. It SHALL play a sound and update the stamp only when the index is positive, the sound is ready and the stamp plus 200 is below the clock.

#### Scenario: The select
- **WHEN** the index is 3, the stamp is 0 and the clock is 300
- **THEN** the sound plays, the stamp becomes 300 and the result is 3
- **WHEN** the index is 0 or the stamp is 200
- **THEN** no sound plays and the stamp stays

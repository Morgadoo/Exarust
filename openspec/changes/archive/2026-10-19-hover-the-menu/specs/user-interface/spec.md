## ADDED Requirements

### Requirement: Menu hover
The menu SHALL take its hover value when it equals field 0x204 and zero otherwise. An unchanged selection SHALL return 0. A change SHALL store the selection, play a sound when it is non-zero and the last stamp plus 200 is below the clock, update the stamp only in that case, and return 3.

#### Scenario: An unchanged selection
- **WHEN** the hover, field 0x204 and field 0x70 are all 5
- **THEN** nothing changes and the result is 0

#### Scenario: A changed selection
- **WHEN** the hover and field 0x204 are 5, field 0x70 is 0, the stamp is 0 and the clock is 300
- **THEN** the selection becomes 5, a sound plays, the stamp becomes 300 and the result is 3
- **WHEN** the selection is 0
- **THEN** no sound plays and the result is still 3
- **WHEN** the stamp is 200 and the clock is 300
- **THEN** no sound plays and the stamp stays

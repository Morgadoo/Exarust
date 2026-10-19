## ADDED Requirements

### Requirement: Skill tooltip show
A missing row SHALL show nothing. A found row SHALL set the index to object+0xb8 − 1 and the threshold to row+0x38 − 1, add 0x10 to the flags, place itself at the anchor x and nine units plus the anchor y, use a height of 24 units when global 0x2ad3c0 is 0 and 18 otherwise, set visible, make three calls and stamp three times the base plus the offset.

#### Scenario: A found row
- **WHEN** a row is found, the anchor is 7, 8, the object word is 5, the row word is 4 and the unit is 5
- **THEN** the index is 4 and the threshold is 3
- **AND** the place is 7, 53 and the height is 120
- **AND** the flags gain 0x10, visible is true and three calls run
- **AND** the stamp is three times 10 plus 20, which is 50

#### Scenario: A missing row
- **WHEN** no row is found
- **THEN** nothing shows and the calls do not run

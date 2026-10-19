## ADDED Requirements

### Requirement: Platform draw
A shown value SHALL be passed with the two platform words. The same value, a zero value, and a block word SHALL not pass it. The indirect call is counted. Its target is not called.

#### Scenario: Value 42 is drawn with the two words
- **WHEN** a match begin shows value 42 with owner 7 and extra 8
- **THEN** those three words are the draw, and the show is counted once

### Requirement: World side
An open world SHALL set the side to 1 when its byte is 0, and to 0 otherwise, before the later select. A lock SHALL leave that side. Otherwise a force SHALL use 0, and a differing byte SHALL be stored, select its bank, clear a span and mask the flags when it is not 0, and be written back. Callback registration is not part of this step.

#### Scenario: A world byte selects the bank
- **WHEN** a match begin clears a mark with world byte 0 and a set flag, byte 5 and full flags, byte 5 forced, and byte 0 locked
- **THEN** byte 0 copies the first bank and installs twice, byte 5 copies the second bank and masks the flags, the force leaves byte 5 and selects the first bank once, and the lock leaves byte 0 while selecting the second bank

### Requirement: Session aside
Passing 1 SHALL store 1, set a flag to 1, and set a code to 0 when the mark is not already 1. An open world SHALL clear a quiet byte and the flags, and store code 4 on the child. Three notice bytes SHALL become 0. Index 0 SHALL be installed and neither bank copied. A hold byte SHALL become 0, a view word SHALL gain bit 0x10, and a view code SHALL become 0. A mark that is already 1 SHALL change nothing.

#### Scenario: Raising the mark installs index 0
- **WHEN** the mark is 0, the current index is 5, and the fallback is 9, and the raise is repeated
- **THEN** the mark is 1, fallback 9 is drawn, the bank is unchanged, and the second raise leaves the code

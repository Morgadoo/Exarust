## ADDED Requirements

### Requirement: Class confirm
A positive selection SHALL store itself and one less, return 7, play a sound when the sound global is positive, copy the page global and show form 0x10012a870 with flag 0. A selection of zero or less SHALL return 0x40 and do nothing else.

#### Scenario: A positive selection
- **WHEN** the selection is 3 and the sound global is positive
- **THEN** the selection is stored, the global becomes 2, the sound plays, the page copies, the form shows and the result is 7

#### Scenario: The guard
- **WHEN** the selection is 0 or negative
- **THEN** the result is 0x40 and no form shows

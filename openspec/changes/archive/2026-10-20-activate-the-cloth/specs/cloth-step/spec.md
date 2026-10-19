## ADDED Requirements

### Requirement: Cloth activation
Activation SHALL OR the four flag bits, set phase 2 and scale 0.4, write the handle when present, copy the root origin and add 0x8000 when the slow global is set.

#### Scenario: The activation
- **WHEN** all four flag words are 0, a handle exists, the object is the root and the slow global is set
- **THEN** the words become 0x20000000, 0x10000, 0x204030 and 0x6000
- **AND** the phase is 2 and the scale is 0.4
- **AND** the handle and origin write once and the slow bit 0x8000 is added

### Requirement: Cloth transform
The transform SHALL make three calls per particle, then the object origin on flag 0x20 and the object extent always.

#### Scenario: Three particles
- **WHEN** the count is 3 and the object flags carry 0x20
- **THEN** nine particle calls, one origin call and one extent call run
- **WHEN** the flags lack 0x20
- **THEN** no origin call runs and the extent still does

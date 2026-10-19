## ADDED Requirements

### Requirement: Cloth start
Start SHALL add 0x10000 to the flags, copy the seed, install a 4×4 identity matrix and set the step to the square root of the count times 1e-6. A body whose flags carry 0x10000000 SHALL set the link bit and be stored.

#### Scenario: The start
- **WHEN** the count is 10000, the seed is 7 and the body carries 0x10000000
- **THEN** the flags gain 0x10000 and the seed is 7
- **AND** the matrix is the identity
- **AND** the step is 0.1
- **AND** the link bit is set and the body is stored
- **WHEN** the body lacks the bit
- **THEN** the link bit is clear and no body is stored

### Requirement: Cloth frame swap
A pending count of at most one SHALL copy the source into both spare buffers twice, clear the count and set flag 1. A larger count SHALL rotate the three buffers and store the frame index.

#### Scenario: The pending copy
- **WHEN** the pending count is 1 and the frame is 9
- **THEN** two copies run, the count becomes 0, flag 1 is set and no frame index is stored
- **WHEN** the count is 2
- **THEN** three buffers rotate and the frame index is 9

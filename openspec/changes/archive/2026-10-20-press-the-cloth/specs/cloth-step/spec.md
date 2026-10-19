## ADDED Requirements

### Requirement: Cloth pressure
The pressure pass SHALL skip on gate 0x100. A normal under 1e-6 SHALL become zero with offset −1. Otherwise it SHALL normalize the normal, store the offset as its dot with the edge plus its length, remove the edge's normal component and scale or subtract it, then move the normal by the edge times the negative length. Afterwards it SHALL run coupled−1 calls, zero the normals and clear 0x1fe.

#### Scenario: One normal and one degenerate particle
- **WHEN** the gate is 0, the coupled word is 3, the first normal is zero and the second is (3,0,0) with edge (0,2,0) and e0 (0,2,5)
- **THEN** the first normal is zero with offset −1
- **AND** the second offset is 3 and its normal is about (1,−6,0.94868)
- **AND** its edge becomes about (0,2,−0.31623)
- **AND** two coupled calls run, the normals are zeroed and 0x1fe clears

#### Scenario: The gate
- **WHEN** the gate word carries 0x100 in its 0xf00 field
- **THEN** the pass returns without particle work or coupled calls

## MODIFIED Requirements

### Requirement: Texture LOD bias
The LOD bias SHALL be the base times (the supersampling factor times the single-precision third), evaluated in that order, where the factor is the supersampling level below 2 and that level less one from 2 up. Every material path, painted, race-skin and run-time apparel included, SHALL take the bias, and a texture settings change SHALL rewrite the bias of every loaded material.

#### Scenario: Supersampling factor
- **WHEN** the supersampling level is 1, 2 and 4 at a bias base of −0.75
- **THEN** the biases are −0.25, −0.25 and −0.75

#### Scenario: A quality change
- **WHEN** the bias changes to −0.25 with two materials loaded, one already at −0.25
- **THEN** both read −0.25 and one changed

## ADDED Requirements

### Requirement: Cube uploads
An image whose creation word picks the cube target SHALL upload six faces with a cube view, each face from the detail level with its stored levels back to back, or its generated chain when it stores one level. A block-compressed cube SHALL be refused.

#### Scenario: A two-level cube
- **WHEN** a 2×2 RGB8 cube with two stored levels is uploaded at detail 0, and at a detail that asks for level 2
- **THEN** it has six layers of two levels, and then six 1×1 layers holding each face's smallest level

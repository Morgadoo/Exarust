# Spec Delta

## MODIFIED Requirements

### Requirement: Terrain composite pass keys
The system SHALL key a composite pass as `layers | textured<<4 | masked<<8 | add<<28 | base_normal<<29` over groups of four layers, and give a node at level L (signed; (0.5, 0.5) when L ≤ 0) of a terrain with block level T the `TexPos` `(2^(L−1) − (bx >> s), 2^(L−1) − (by >> s))` with `s = (T − L) mod 32` taken as a wrapping 32-bit difference.

#### Scenario: An additive group
- **WHEN** a normal pass covers six layers
- **THEN** its second group of two layers carries the additive bit

#### Scenario: A node deeper than the block level
- **WHEN** a level-4 node at column 7, row 5 of a block-level-3 terrain takes its `TexPos`
- **THEN** the shift count is 31 and the coordinate is (8, 8) with third component 16/4800

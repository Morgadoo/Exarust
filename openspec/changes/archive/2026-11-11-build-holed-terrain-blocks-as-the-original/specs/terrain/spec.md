## ADDED Requirements

### Requirement: Holed terrain block triangles
A terrain block whose tile hole mask removes any of its 1800 triangles SHALL draw and collide with the list the original's block activation builds: each cell split along the diagonal from its low vertex plus one to its low vertex plus one row, the first triangle kept only when none of its three vertices is a hole and the second likewise. A block with every triangle SHALL keep the shared patch.

#### Scenario: One hole
- **WHEN** a single vertex inside a block is a hole
- **THEN** the block draws 1794 triangles, none of them touching that vertex

#### Scenario: No hole
- **WHEN** no vertex of a block or its far edge is a hole
- **THEN** the block draws the shared patch

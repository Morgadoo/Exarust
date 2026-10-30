# terrain-manager Specification

## Purpose
The terrain manager's tile record, block trees and static set-up.

## Requirements

### Requirement: Tile record
The terrain manager model SHALL size a tile as 240 cells of 20 cm over 241 vertices and 244 apron rows, hold positions and normals as the two halves of a 24-byte-per-vertex buffer, and compute height bounds over the 16 x 16 vertex blocks every 15 vertices.

#### Scenario: Block trees
- **WHEN** the manager is created
- **THEN** it holds the shared 8 x 8 translated block trees

### Requirement: Static set-up
The static terrain set-up SHALL mark the scene only when the manager is created, build 64 shadow receivers per tile outside room mode and size a pool in room mode.

#### Scenario: Reuse
- **WHEN** the static terrain is created twice
- **THEN** the second call reuses the manager and does not mark the scene again

### Requirement: Block descriptor
The terrain manager model SHALL give a block descriptor a vertical half extent of half the block's height span plus one centimetre with no other minimum, and SHALL walk a block tree as a node's descriptor followed by its four children's in order.

#### Scenario: A flat block
- **WHEN** every bounds cell of a block is at 5.0
- **THEN** the descriptor's vertical centre is 5.0 and its vertical half extent is 1.0

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

### Requirement: Tile read-back hand-off
Queuing a tile's bake SHALL clear the record's bit 0x100. With a pending record, the hand-off SHALL copy the 244² height plane (59 536 bytes) when the renderer's feature bit 0x2000 is set and the 240² × 3 packed plane (172 800 bytes) when bit 0x1000 is set, then clear the record's bit 0x2000, set its bit 0x4000 and empty the pending slot; with none pending it SHALL do nothing.

#### Scenario: Both planes
- **WHEN** a pending record is handed off with both feature bits set
- **THEN** 0xe890 height bytes and 0x2a300 packed bytes are copied and the record gains 0x4000 and loses 0x2000

### Requirement: Block node
The terrain unit SHALL allocate a tile record's block node as a single zero-filled root with no children (the whole-tile descriptor) and SHALL NOT build the 85-node quarter tree itself; the descriptor walk SHALL visit only the children some other unit attaches.

#### Scenario: Update
- **WHEN** a record built by the terrain unit alone has its height bounds changed
- **THEN** only the whole-tile descriptor is recomputed

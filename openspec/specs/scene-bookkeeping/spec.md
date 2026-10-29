# scene-bookkeeping Specification

## Purpose
The live scene's node-removal queue and the scene editor's frame step, as the original's machine code does them.

## Requirements

### Requirement: Removal queue
The scene bookkeeping model SHALL keep a 256-entry node-removal queue of which 255 are usable, and a full queue SHALL still mark the node and the sector's removal bits.

#### Scenario: A full queue
- **WHEN** a 256th removal is queued
- **THEN** the node and the sector's removal bits are still marked

### Requirement: Frame step
The scene editor model SHALL take the frame step as a signed 64-bit stamp difference compared in double with 16.666666666666668 and store a capped step as the float 0x41855555.

#### Scenario: A long frame
- **WHEN** a frame lasts longer than the limit
- **THEN** the stored step is the capped float

### Requirement: Removal drain
Each frame SHALL first move every queued removal, oldest first, to the end of a pending list and hang its node from the global world. At the frame end a finished entry SHALL be replaced by the last pending one, which is looked at next.

#### Scenario: Swap order
- **WHEN** deletions A, B, C and D are pending and B's subtree is owned
- **THEN** A, D and C are taken out in that order and B stays pending

### Requirement: Removal kinds
At the frame end a hidden node SHALL get its owner back. A deletion SHALL wait while its subtree is marked owned, otherwise get its owner back, be reported when that owner is the global world, and be taken out with its subtree, or deferred to the free list while another owner holds it. A free entry SHALL skip the wait.

#### Scenario: A hidden node
- **WHEN** a node in a room is queued hidden
- **THEN** it hangs from the global world during the frame and belongs to the room again after it

#### Scenario: A held free entry
- **WHEN** an owned node and a held node are queued as free entries
- **THEN** the owned node is taken out at once and the held node goes to the free list without getting its owner back

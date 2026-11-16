## ADDED Requirements

### Requirement: Vegetation only on built blocks
The plant generator SHALL walk a tile's built blocks in chunk order (row by row) in both of its passes: a 6 m block with no block node SHALL grow no plants and SHALL draw no random numbers, so the blocks after it take the draws it would have taken. A built block whose points are all too sparse SHALL still draw its four numbers a point.

#### Scenario: A missing block
- **WHEN** a tile's first block has no node and the rest grow at full density
- **THEN** nothing grows in that block and the next block's plants differ from those of a tile where the first block is built

## ADDED Requirements

### Requirement: Block culling box
Each spawned 6 m terrain block SHALL cull by its leaf descriptor over the tile's height bounds: the block's centre and half extent across, and the height range of its bounds cells plus 1 cm on the half height. A block on the tile's edge SHALL reach down over the skirt below it.

#### Scenario: Sloped block
- **WHEN** a block's vertices rise from 0.60 m to 0.90 m
- **THEN** its box is centred at 0.75 m with a half height of 0.16 m

#### Scenario: Edge block
- **WHEN** a block lies on its tile's edge
- **THEN** its box's bottom is lowered by the skirt depth

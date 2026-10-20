# Spec Delta

## ADDED Requirements

### Requirement: GI block warm-up
The system SHALL keep the block already on screen bound while a rebuilt voxel
GI block warms behind it, and SHALL swap the new block in only after its full
warm-up has run. A level change SHALL drop both blocks.

#### Scenario: Rebuild while walking
- **WHEN** the camera's GI focus moves far enough to rebuild the block
- **THEN** the floor keeps the old block's ambient while the new one warms instead of showing black patches or a circular halo

#### Scenario: First block on load
- **WHEN** a level loads and its first GI block is built
- **THEN** that block is bound immediately and the loading screen waits for its warm-up, as before

#### Scenario: New level
- **WHEN** a different level starts
- **THEN** the previous level's active and pending GI blocks are dropped before the new level's first block is bound

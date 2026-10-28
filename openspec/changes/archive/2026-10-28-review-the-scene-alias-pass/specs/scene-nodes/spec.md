## ADDED Requirements

### Requirement: Membership rebuild
The system SHALL drop the aliases a sector does not own as the original does: an unprocessed one is replaced by the last alias; a processed one's slot is taken by the last kept, not yet moved alias after it and its owner, if any, is marked as links changed; the unprocessed aliases follow the compacted ones. Kept processed bucket meshes SHALL have their bucket chains remapped and linked to the waiting aliases.

#### Scenario: A hole filled from the end
- **WHEN** a sector's processed aliases are A, B, C, D, B is owned by another sector, and E waits
- **THEN** the aliases are A, D, C, E, the other sector is marked, and a mesh's chain visits A, C, D, then E

### Requirement: Dropped meshes
The system SHALL drop the meshes a sector no longer owns; a dropped processed mesh SHALL lower the processed count and give its bucket faces to the kept aliases' shadow maps as needed faces.

#### Scenario: A dropped mesh
- **WHEN** a processed mesh with bucket faces 0x9 and 0x4 leaves a sector whose first alias has a shadow map needing 0x2
- **THEN** that shadow map needs 0xb and the second alias, without a shadow map, is unchanged

### Requirement: Light activation
The system SHALL activate a light whose state word is below 2: state 2, its position snapshotted unless its light word skips it (height 0 in the flat snapshot mode), its sector alias found or created (owned by the sector, moved); in grid mode its cell SHALL be stored and the flood or fill reported, otherwise an alias relink. A light out of the scene and the sector SHALL drop all its aliases.

#### Scenario: A flat snapshot without a grid
- **WHEN** a flickering light at (4, 2.5, 1) m in the flat snapshot mode activates in a scene without a grid
- **THEN** its snapshot is (4, 0, 1), an alias relink and a first flicker step are reported, and no cell flood

### Requirement: Scene list cleanup
The system SHALL free an ownerless scene on release and list one with an owner; the cleanup hook SHALL free the listed scenes not marked owned and then run the previous hook.

#### Scenario: Release and cleanup
- **WHEN** scene A and scene B are released with owners, B is marked owned, and C is never released
- **THEN** the cleanup frees A and keeps B and C

# level-decoding Specification

## Purpose
Reads the level-specific sections of the original chunk-tree files so a level
can be assembled: which modular sets it uses, its tile grid, and the objects
placed in it.

## Requirements

### Requirement: Tile-grid header
The system SHALL decode a level's tile-grid section into its wall-set names, floor-set names, grid width and height, and the three per-cell value planes.

#### Scenario: First story level
- **WHEN** `exanima01.rfc` is decoded
- **THEN** the grid is 32 by 40 cells, uses wall sets `GEWS_UWA01` and `GEWS_UwA01p` and floor sets `GEFS_UWA01` and `GEFS_UWA01T`, and each plane holds 1280 values

### Requirement: Placed props
The system SHALL decode every placed prop of a level into the object name, its placement transform in metres, and its parent index.

#### Scenario: Arena props
- **WHEN** `arena.rfc` is decoded
- **THEN** 19 props are returned and each names an object present in `Objlib.rpk`

### Requirement: Level coverage
The system SHALL decode the grid and props of every level in `Resource.rpk` without failing, reporting unsupported sub-records instead.

#### Scenario: Corpus run
- **WHEN** every level file is decoded
- **THEN** all grids and props decode and the share of props whose object exists in the object library is reported

### Requirement: Placed characters
The system SHALL decode the characters a level places, with their records and worn items from the level's own object database, and SHALL spawn placed humans where the level puts them, wearing their items painted with their appearances.

#### Scenario: First story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the level's human inhabitants stand where the level places them, dressed in their own clothes, and its dead lie down

### Requirement: Placed objects
The system SHALL decode the object-database objects a level places and spawn each with its item's model at its transform.

#### Scenario: First story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's placed objects appear, and none of their models are missing

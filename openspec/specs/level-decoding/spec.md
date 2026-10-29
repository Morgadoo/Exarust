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
The system SHALL decode the characters a level places, with their records and worn items from the level's own object database or the game's, and their races from `races.rdb`. It SHALL spawn each in its race's body playing its race's moveset where the level puts them. Characters of the human frame SHALL wear exactly the items their records list, including those without a ground model, each as the model its body names and painted with its appearance.

#### Scenario: First story level
- **WHEN** `exarust play exanima01` runs
- **THEN** the level's human inhabitants stand where the level places them, dressed in their own clothes, and its dead lie down

#### Scenario: Braies and quilted trousers
- **WHEN** `exarust play exanima01` runs
- **THEN** Sticky and the two dead men wearing braies show pale grey braies, Patient and Jimmy wear their quilted trousers, and no placed human is given an item its record does not list

#### Scenario: Golem hall
- **WHEN** `exarust play exanima06` runs
- **THEN** its golem guards stand in their golem bodies, and no placed character lacks a body

#### Scenario: Corpus
- **WHEN** every level in `Resource.rpk` is decoded
- **THEN** each of the 103 placed humans wears an item that covers the groin, and all 481 items they wear name a model in `Characters.rpk`

### Requirement: Placed objects
The system SHALL decode the object-database objects a level places and spawn each with its item's model at its transform.

#### Scenario: First story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's placed objects appear, and none of their models are missing

### Requirement: Placement transform records
The system SHALL apply a placed prop's or object's transform records before its bodies register: each is its node's frame in the level, taken into the node's local transform from its parent's frame at the placement, then its nested records apply to that node's children. A record SHALL find its node from the one it applies to (the root at the top level): that node for an empty name, else its first direct child of exactly that name, and is skipped with its nested ones when that node is missing.

#### Scenario: First story level
- **WHEN** `exanima01.rfc`'s props are read against their objects
- **THEN** every record finds its node and some transform records carry nested ones

#### Scenario: A wall ring
- **WHEN** a wall ring placed turned half round has a record putting the ring 10 cm along the model's z from the placement
- **THEN** the ring's local transform is 10 cm along z and its frame in the level is the record's

### Requirement: Placement position records
The system SHALL apply a placement's position record to a mesh node when its count is the node's position-stream count, rebuilding the object's model with the new elements and normals recomputed from them.

#### Scenario: Stored banner drapes
- **WHEN** `exanima01.rfc`'s position records are read against their objects
- **THEN** all 14 match their meshes' position-stream counts

### Requirement: Node kinds and instances
The system SHALL read camera nodes (the node they follow and one word) and sector nodes (no body), keep the header's instance word, and draw an instance node with its owner's meshes, lights and emitters at its own transform; an instance node stores no body or sub-chunks. The mesh body reader SHALL replace a mesh's deprecated fading bit (bit 31) by bit 24; other node kinds keep the word.

#### Scenario: Instanced beams
- **WHEN** a level whose nodes instance other nodes is shown
- **THEN** each instance draws its owner's mesh where the instance stands

### Requirement: Level strings and database-only read
The system SHALL read a level's own string table with its local database, report a table that does not decode while the rest of the database loads, and read a level's local records and strings on their own without building its scene.

#### Scenario: Database only
- **WHEN** only a level's database is read
- **THEN** its object records and string table are returned and its placed objects are not read

### Requirement: Character locale and material order
The system SHALL create a world-database character only in the level whose locale its record stores (a level's own records always), and SHALL shade a surface with a library material over a file's own record of the same name.

#### Scenario: Locale filter
- **WHEN** a level places a world-database character whose stored locale is another level's
- **THEN** that character is not created there

### Requirement: Light animation parameters
A light body whose flags carry 0x10000 or 0x20000 SHALL be read with four more words after its shaping words: one skipped, then the flicker amplitude, speed and emissive share, and the node's sub-chunks after them. Other light bodies SHALL carry no parameters.

#### Scenario: A flickering light
- **WHEN** a light body with flag 0x10000 ends with the words 0, 0.25, 0.5, 0.75 and a key-track sub-chunk
- **THEN** its amplitude is 0.25, its speed 0.5, its emissive share 0.75, and the key track is read

### Requirement: Grid border fill
Every grid read SHALL write plane A's border words in the loader's order: `0x20010007` over the whole top row, `0x00120070` and `0x12007000` down the left and right columns, `0x01200700` over the whole bottom row, then the corners `0x20130077`, `0x32017007`, `0x01320770` and `0x13207700`; an empty grid SHALL be left alone.

#### Scenario: A 4 × 3 grid
- **WHEN** a 4 × 3 zero plane A is filled
- **THEN** its rows are `20130077 20010007 20010007 32017007`, `00120070 0 0 12007000` and `01320770 01200700 01200700 13207700`

#### Scenario: One row
- **WHEN** a 3 × 1 grid is filled
- **THEN** it holds `01320770 01200700 13207700`

### Requirement: Placement tree entries
A `0x3DE0EC00` tree entry SHALL apply to the first direct child whose name equals the entry's (an empty name matching only an unnamed child), its nested entries to that child's children, and an entry whose child is missing SHALL be skipped with its nested entries; each record's next sibling SHALL follow its `(descendants + 1) × 0x4c` stride.

#### Scenario: An empty name under a named child
- **WHEN** a tree moves `Arm` with a nested empty-name entry, and an empty-name entry at the top, over a root whose `Arm` has an unnamed child
- **THEN** `Arm` and its unnamed child move and the root does not

### Requirement: Sub-chunk registry mode
With a handler registered, the reader SHALL hand every node sub-chunk to the first handler of its tag in place of its own sub-chunk handling; root chunks SHALL never reach the registry, and an empty registry SHALL read as the plain reader.

#### Scenario: A sector's sub-chunk
- **WHEN** a sector node carries sub-chunk `0x12345678` and the file a root chunk of the same tag, and a handler is registered for it
- **THEN** the handler sees the sector's body once, the node records no unknown sub-chunk, and the plain reader records the tag on the node

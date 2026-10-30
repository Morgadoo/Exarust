## ADDED Requirements

### Requirement: Procedural pass tiles
The system SHALL compute `MatTile` as tile size times region size over the layer material's first colour texture size and `DmgTile` as tile size times region size over 1024, where the tile size is the job size, doubled below texture detail 3.

#### Scenario: Low-detail tiles
- **WHEN** a region of size (0.5, 0.5) is drawn into a 256 by 128 job at texture detail 2 against a 256 by 256 texture
- **THEN** `MatTile` scales are (1, 0.5) and `DmgTile` scales (0.25, 0.125)

### Requirement: Procedural shade tile
The system SHALL bind `ShdTile` as the region rectangle (size, offset) when the region has its own shade tile and as (1, 1, 0, 0) otherwise, with its y multiplied by `UVScale` in every pass; `PtrnPos` SHALL be bound only in the painted-pattern pass and `NrmTile`, the unfolded region rectangle, only in the specular pass.

#### Scenario: A stretched region
- **WHEN** a region of size (0.5, 0.5) at offset (0.25, 0.5) with `UVScale` 0.5 is drawn
- **THEN** `ShdTile` is (0.5, 0.25, 0.25, 0.5) in the colour, pattern, specular and normal passes, and the specular `NrmTile` is (0.5, 0.5, 0.25, 0.5)

### Requirement: Factory paint targets
The system SHALL paint a factory part at its atlas region's texel size (rounded to nearest, magnitude of a negative scale), halved by integer division below texture detail 3, and SHALL draw only the target columns whose centre lies left of the region's width crop (1 − texture parameter) times the width, sampling the source at an unscaled u.

#### Scenario: Low detail
- **WHEN** a 512 by 256 region is painted at texture detail 2
- **THEN** the target is 256 by 128

#### Scenario: A cropped region
- **WHEN** a region with crop 0.5 is painted 4 texels wide
- **THEN** columns 0 and 1 paint as without the crop and columns 2 and 3 keep the cleared colour and surface (0, 0.01, 0)

### Requirement: Factory repaint
The system SHALL keep a factory weapon's assembled meshes when only its condition changes and repaint them from the same seeds and stored choices.

#### Scenario: Damage only
- **WHEN** a weapon with the same seeds is loaded again with a higher damage byte
- **THEN** its mesh handles are the same and its materials differ

### Requirement: Paint job ring and surface cache
The system SHALL model the 256-slot paint-job ring (a push stores first and a full ring flushes every job but the new one) and the surface cache of 64 slots with a separate pool of 64 >> 4 for extra-layer jobs, where a get scans from the read cursor and a return to a full ring frees the entry at the write cursor.

#### Scenario: A full ring
- **WHEN** 256 jobs are pushed into an empty ring
- **THEN** the last push runs 255 jobs and the 256th stays queued

### Requirement: Factory impact sound
The system SHALL give a factory weapon the hit sound of the factory's table by its section's sound byte (`WH_SwordA`, `WH_MaceMA`, `WH_AxeA`, `WH_HammerA`, `WH_MaceMA`, `WH_ClubMW`, `WH_WoodA`, `WH_ClawsL`, `BdyCol`), and `WH_SwordA` without a section.

#### Scenario: Class 3
- **WHEN** the sound byte is 3
- **THEN** the sound is `WH_HammerA`

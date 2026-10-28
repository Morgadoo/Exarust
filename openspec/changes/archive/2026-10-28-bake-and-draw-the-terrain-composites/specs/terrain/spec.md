# Spec Delta

## ADDED Requirements

### Requirement: Terrain composite bake
The system SHALL, when `EXA_TERRAIN_SETS=1`, bake at most one held composite set a frame, in selection order, running the compositor's normal, colour, gloss and displacement passes in that order into the set's targets, each a full-target quad over the block.

#### Scenario: One bake at a time
- **WHEN** two blocks hold hosted sets and neither is baked
- **THEN** only the block first in the selection is queued, and the second is queued once the first bake is reported done

### Requirement: Terrain composite bake content
Each bake pass SHALL write the per-pixel layer stack at its texel — normal xy as xy·½ + ½, colour before the terrain's 0.75 diffuse factor, gloss, parallax height — clamped to the 8-bit target; the displacement target SHALL be half the set's side and the colour target SHALL store sRGB.

#### Scenario: Matching the per-pixel draw
- **WHEN** `arenao1` is drawn with a fixed scene seed, once from the baked sets and once per pixel
- **THEN** the two frames differ by about as much as two per-pixel runs do

### Requirement: Terrain composite draw
The system SHALL draw a block from its set only after its bake has run, reading the set at the tile-local position in centimetres times `TexPos.z` plus `TexPos.xy`, shifted by the set's height in four accumulating parallax steps of `vt.xy·0.007·(4 + 3·vt.z)·(h − ½)` and eased back by min((h₀ − h₃)·2, 1); a block that loses its set, or whose set's targets are rebuilt, SHALL return to the per-pixel draw.

#### Scenario: The composite coordinate spans the block
- **WHEN** block (2, 5) of a 48 m tile centred at (100 m, −40 m) is drawn from its set
- **THEN** its composite coordinate is (0, 0) at world (88 m, −34 m) and (1, 1) at (94 m, −28 m)

#### Scenario: A lost set
- **WHEN** a block drawing from its set no longer holds it
- **THEN** the block draws its tile's per-pixel material again

### Requirement: Terrain room light rank
The system SHALL rank a hidden, in-reach block in a room against that room's light list only — each light joining the room of the cell it stands in, then every room that owns a light passing its whole list through its portals once, in room order — leaving 65000 cm when the list is empty, and SHALL rank a block in no room, or in a level without rooms, against every light.

#### Scenario: A room without lights
- **WHEN** a hidden block's room lists no light though a light in another room overlaps it
- **THEN** its light distance is 65000

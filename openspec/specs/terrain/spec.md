# terrain Specification

## Purpose
Decodes the original terrains and their palettes and shows them as walkable
ground textured like the original.

## Requirements

### Requirement: Terrain decoding
The system SHALL decode every shipped terrain into tiles of heights, hole masks and painted layers, and every palette into materials, blend rules and plants, returning errors instead of panicking on malformed data.

#### Scenario: Corpus
- **WHEN** `exa-tool corpus terrain` runs over the installed game
- **THEN** all terrains and palettes parse without errors

#### Scenario: Truncated tile
- **WHEN** a terrain's tile chunk is shorter than its declared size
- **THEN** parsing fails with an error

### Requirement: Blend rules
The system SHALL resolve painted layers into palette material weights per vertex by evaluating the palette's blend rules, with leftover weight sent to the first rule.

#### Scenario: Height rule
- **WHEN** a rule of kind 0 has bounds 0 m and 2 m and a vertex lies at 1 m
- **THEN** half of the weight goes to each child

#### Scenario: Cyclic rules
- **WHEN** two rules name each other as children
- **THEN** evaluation terminates

### Requirement: Terrain in levels
The system SHALL spawn a level's terrain beside its other content, positioned on the level's tile grid of 48 m tiles, with holes left open and collision for characters.

#### Scenario: Outdoor arena
- **WHEN** `exarust play arenao3` runs
- **THEN** the player stands and walks on the terrain inside the arena

### Requirement: Height pass
The system SHALL displace terrain vertices by the palette materials' height maps, layered with the tile's opacities and blend masks, as the original's height pass does.

#### Scenario: Neutral ground over a dungeon floor
- **WHEN** neutral terrain is painted over a dungeon floor at the same height
- **THEN** the terrain rests just above the floor instead of z-fighting with it

### Requirement: Vegetation
The system SHALL grow grass and plants on terrain where the palette materials' vegetation density allows, choosing, sizing and turning each from the tile's stored seed as the original does, standing on the displaced ground.

#### Scenario: Grassland
- **WHEN** an outdoor arena with grass materials is shown
- **THEN** grass tufts and plants stand on its grassy areas and none grow on bare dirt or rock

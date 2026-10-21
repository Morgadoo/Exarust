# Spec Delta

## ADDED Requirements

### Requirement: Terrain palette versions
The system SHALL read terrain palettes of versions 2 and 3 into the original's runtime material record, a version 2 material gaining the parameters 0.5, 0.3, 0.5 and 1.0 at index 20 (its stored parameters 20–24 moving to 24–28). A level SHALL load `default.ftb` when its terrain's palette is missing, and only then.

#### Scenario: Shipped palettes
- **WHEN** every palette in `Resource.rpk` is parsed
- **THEN** `default.ftb` and `uwa1b.ftb` (version 2) give 29 parameters per material with 0.5, 0.3, 0.5, 1.0 at index 20, and the others (version 3) are read as stored

#### Scenario: Missing and broken palettes
- **WHEN** a terrain names a palette that is not in the pack
- **THEN** "Could not find <name>.ftb" is logged and `default.ftb` is used; a palette that is there but does not parse is not replaced

### Requirement: Version 1 palette upgrade
The system SHALL upgrade a version 1 palette as the original's loader does: the old textures moved to the diffuse, normal and height slots, a mask named from the diffuse name when the old height amount is above `f32::EPSILON`, the old tint times 1.4, the negated offset (1 below epsilon), parameters 20–28 zero, an old shader grass renamed to the `Trn_GrasA01` set, and three default plants.

#### Scenario: Upgraded material
- **WHEN** a version 1 material with a height amount of 0.5 and diffuse texture `Dtex` is read
- **THEN** its mask texture is `DteM`, parameters 2–5 are 3, 1, 3 and 1.5, and the palette's plants are `Trn_GrassA01`, `Trn_SPlntA01` and `Trn_BPlntA01`

### Requirement: Vegetation lookups and drawing
The system SHALL sample a plant point's density from the 240 × 240 mask at texel `round(p × 0.9958333)` (ties to even) as the mask byte times the double 1/255 less 0.2, rounded to `f32`, with −1 where the nearest vertex is a hole. Grass and small plants SHALL be lit as facing up and drawn without a vertex tint. Each 6 m chunk SHALL belong to its own block's room, have no collider, and cast shadows only above shadow quality 4.

#### Scenario: Default shadow quality
- **WHEN** an outdoor arena is shown at the shipped shadow quality 4
- **THEN** its grass and plants cast no shadows, and characters walk through them without colliding

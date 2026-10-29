# Spec Delta

## MODIFIED Requirements

### Requirement: Terrain in the GI volume
The system SHALL fill the GI occupancy below the terrain surface, as the original does, so light does not drain beneath terrain floors. With k the surface height in voxels truncated towards zero, a column with k below 0 SHALL be skipped; for k below the top slice voxel k SHALL hold round-half-even(fraction × 20) and slices 0..k−1 SHALL hold 20; otherwise k SHALL clamp to the top slice, no surface byte SHALL be written and slices 0..D−2 SHALL hold 20. The column's lowest slice SHALL be k.

#### Scenario: Room on terrain
- **WHEN** a torch burns in a room whose floor is terrain
- **THEN** its bounce light stays in the room and reaches surfaces in shadow

#### Scenario: Terrain above the lid
- **WHEN** a terrain column of a 64-slice block reaches voxel 67
- **THEN** slices 0 to 62 hold 20, slice 63 keeps the lid's 26 and the column's lowest slice is 63

#### Scenario: Terrain just under the top slice
- **WHEN** a terrain column of a 64-slice block reaches voxel 62.5
- **THEN** slice 62 holds 10, slices 0 to 61 hold 20 and slice 63 keeps 26

## ADDED Requirements

### Requirement: GI program bindings
The system SHALL keep the original GI manager's program table: `gi_inject_v` and `gi_inject_g` linked with `gi_inject_f`, `gi_emission` or `gi_blit_f`; the inject program binding light i's `Shadow<i>` to texture unit 2 + i, `LVox` to 0 and `OVox` to 1; the emission program binding only `OVox`, to 1; the blit program binding `LVox` to 0.

#### Scenario: Twelfth light
- **WHEN** the inject program binds the twelfth light of a group
- **THEN** it names `Shadow11` and binds it to unit 13

### Requirement: GI base ambient quality
The system SHALL scale the level record's secondary ambient by 0.001 for the GI base ambient at shading quality 4 or above, and by 0.002 below it.

#### Scenario: Low shading quality
- **WHEN** a level's secondary ambient is 40 and shading quality is 3
- **THEN** the GI base ambient is 0.08 per channel

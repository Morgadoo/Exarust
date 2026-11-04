# Spec Delta

## MODIFIED Requirements

### Requirement: GI sky visibility
A voxel SHALL see the GI sky light when it is at or above its column's sky floor and its centre lies above −16 m: the floor is the slice above the column's highest voxel below the top slice with an occupancy byte of 10 or more, or slice 0 for a column without one. The top slice's soft lid SHALL NOT count as an occluder.

#### Scenario: Under a roof
- **WHEN** a column has a roof voxel at slice 2 above an open voxel
- **THEN** the sky light reaches slices 3 and up of that column and not the voxel under the roof

#### Scenario: Open column under the lid
- **WHEN** a column holds only the block's lid (26 in the top slice, 5 below it) above ground at slice 0
- **THEN** the sky light reaches slice 1 and up

#### Scenario: Below the sky map's depth window
- **WHEN** an open voxel's centre lies at −16.1 m
- **THEN** it takes no sky light

### Requirement: GI base ambient quality
The system SHALL scale the environment's fill colour by 0.001 for the GI base ambient at shading quality 4 or above, and by 0.002 below it, taking the shading quality from the settings each frame: the level record's secondary ambient without a sky, and under an open sky the day record's fill as the last sun placement left it, every frame. At quality 4 or above the manager's light share SHALL be 0.1 times the double 0.9, narrowed to single precision.

#### Scenario: Low shading quality
- **WHEN** a level's secondary ambient is 40 and shading quality is 3
- **THEN** the GI base ambient is 0.08 per channel

#### Scenario: Open sky at noon
- **WHEN** an outdoor arena's sun placement leaves a fill of (0.156, 0.188, 0.223) at shading quality 5
- **THEN** the GI base ambient is (0.000156, 0.000188, 0.000223) per step, whatever the level record's secondary ambient

#### Scenario: High shading quality share
- **WHEN** the shading quality is 5
- **THEN** the light share is the single-precision float nearest 0.09

## ADDED Requirements

### Requirement: GI volume probe
With `EXA_GI_PROBE` set the system SHALL read the camera block's light texture back from the GPU on the frame it names (600 by default) and log, per slice, the mean and largest decoded light over the open voxels of the drawn interior, the light of one column (`EXA_GI_PROBE_AT`, the block's centre by default), the base ambient and sky light colour, each group's lights, and the open sky cube's per-face means at mips 5 and 7. Without it nothing SHALL be read back.

#### Scenario: Probe at the default frame
- **WHEN** a level runs with `EXA_GI_PROBE` empty
- **THEN** frame 600 logs the block's slices and lights once

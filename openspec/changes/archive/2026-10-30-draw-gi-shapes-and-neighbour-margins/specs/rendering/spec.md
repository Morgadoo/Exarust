# Spec Delta

## ADDED Requirements

### Requirement: GI shape injection
The system SHALL add every registered GI shape's fragment body into the light step, per voxel, over the shape's clipped column box and slice range, after the point-light groups and before the voxel lights; the shell bodies SHALL read the voxel's world centimetres through the shape's frame, and the line and multi-point bodies the unframed position.

#### Scenario: Cuboid in the block
- **WHEN** a cuboid shape lies inside the camera's block
- **THEN** the voxels of its clipped box take its colour falloff each step and the voxels outside it take none

### Requirement: GI matrix-shadow slot move
Before a group is drawn the system SHALL move the first light with a 2D matrix shadow at index 2 or later to slot 1 when the sky light is up, or the first at index 1 or later to slot 0 when it is not, scanning to the last light and moving only one.

#### Scenario: Short group
- **WHEN** the sky light is up and a three-light group holds a matrix-shadow light last
- **THEN** that light moves to slot 1

### Requirement: GI resident light textures
The system SHALL keep the light textures of up to nine GI blocks, evicting the least recently used block other than the active one, and a block the camera returns to SHALL reuse its light and skip its warm-up.

#### Scenario: Walking back
- **WHEN** the camera leaves a block and returns before nine others are built
- **THEN** the block's light is as it left it and no warm-up runs

### Requirement: GI neighbour margin copy
The system SHALL copy each resident neighbouring block's light into the active block's margins from the original's table of texture fractions: on the neighbour's side the strip 0..1/32 or 31/32..1 with UV offset +15/16 or −15/16, the interior 1/32..31/32 on the other axis, every slice but the top one.

#### Scenario: Right neighbour of a 256-wide block
- **WHEN** the block at +1 along X is resident for a 256 × 256 × 64 block
- **THEN** columns 248..256 by rows 8..248 over 63 slices are read 240 columns over in the neighbour

#### Scenario: 128-wide block
- **WHEN** the block at +1 along X is resident for a 128 × 128 × 16 block
- **THEN** columns 124..128 by rows 4..124 over 15 slices are copied with offset −15/16

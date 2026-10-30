# Spec Delta

## ADDED Requirements

### Requirement: GI per-object block pick
The system SHALL light each object from the resident GI block its position falls in: the position less the block grid's low edge, in centimetres, times 1/4800, floored on each ground axis, names the block; terrain SHALL use the centre of each 6 m terrain block and plants their block's centre. Surfaces SHALL sample the picked block with no box test, and an object whose block is not resident SHALL take no GI light.

#### Scenario: Ground across a block edge
- **WHEN** the camera's block ends across a shadowed floor and the block beyond it is resident
- **THEN** the floor beyond the edge takes that block's GI light and shows no step at the edge

#### Scenario: No resident block
- **WHEN** an object stands in a block that holds no light texture
- **THEN** its GI ambient is black, as with the original's texture 0

### Requirement: GI resident blocks step
The system SHALL run a light step every frame for every resident GI block, each with its own sector marks, cell lights and margin copies from the resident blocks beside it; a block that becomes resident without kept light SHALL warm up, sharing at most sixteen extra steps a frame with the others.

#### Scenario: Neighbour in view
- **WHEN** the view reaches into a resident neighbouring block
- **THEN** that block's marked sectors are lit by their own lights each frame

## MODIFIED Requirements

### Requirement: Voxel GI shading
The system SHALL light surfaces from the voxel GI volume. The ambient is the volume 20 cm along the normal × the level ambient × 15 × openness. Reflections are the environment cube × that ambient × 16.

#### Scenario: Torchlit room
- **WHEN** a level with torches has been shown for its warm-up
- **THEN** walls and floor out of a torch's direct reach show its warm bounce light

#### Scenario: Outside the block
- **WHEN** a surface lies beyond the GI block its object is lit from, or GI is turned off
- **THEN** it reads the block's clamped edge, or with GI off takes its ambient from the environment cube

### Requirement: GI neighbour margin copy
The system SHALL copy each resident neighbouring block's light into every stepping block's margins from the original's table of texture fractions: on the neighbour's side the strip 0..1/32 or 31/32..1 with UV offset +15/16 or −15/16, the interior 1/32..31/32 on the other axis, every slice but the top one. A block warming behind the camera's SHALL take no copies until it is bound.

#### Scenario: Right neighbour of a 256-wide block
- **WHEN** the block at +1 along X is resident for a 256 × 256 × 64 block
- **THEN** columns 248..256 by rows 8..248 over 63 slices are read 240 columns over in the neighbour

#### Scenario: 128-wide block
- **WHEN** the block at +1 along X is resident for a 128 × 128 × 16 block
- **THEN** columns 124..128 by rows 4..124 over 15 slices are copied with offset −15/16

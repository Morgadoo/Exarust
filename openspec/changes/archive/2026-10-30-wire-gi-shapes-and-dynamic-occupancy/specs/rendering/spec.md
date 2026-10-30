# Spec Delta

## MODIFIED Requirements

### Requirement: GI shape draw box
The system SHALL draw a registered GI shape over its centre in voxels, `width · ((p − block centre) · s + 0.5)` rounded half to even with s = 1/(width · voxel) (the height measured from the world's zero as `width · (below/width + p · s)`), and its half extents `ceil(width · (h · s))`, with the columns clipped to 8 and 248, from the centre less its vertical half extent for twice it plus one slices, and SHALL draw nothing when the centre slice is below 2 or the clipped box is empty.

#### Scenario: Shape near the floor
- **WHEN** a shape's centre lies in slice 1
- **THEN** it draws nothing

### Requirement: GI dynamic occupancy
The system SHALL update a moving GI mesh's occupancy every frame by splatting its old points with weight −20 and then its new points with +20 in each built block the two frames' padded boxes meet, over a region padded 1.5 voxels on each side in world voxels, rounded half to even and ending one voxel short of the block's size; a thin mesh SHALL weigh a quarter and a centred one SHALL add half a voxel to the height and leave its column's lowest slice alone.

#### Scenario: Tie in the padding
- **WHEN** a point lies 4 voxels from the block's corner
- **THEN** its region runs from voxel 2 to voxel 6

### Requirement: GI base ambient quality
The system SHALL scale the level record's secondary ambient by 0.001 for the GI base ambient at shading quality 4 or above, and by 0.002 below it, taking the shading quality from the settings each frame; at quality 4 or above the manager's light share SHALL be 0.1 times the double 0.9, narrowed to single precision.

#### Scenario: Low shading quality
- **WHEN** a level's secondary ambient is 40 and shading quality is 3
- **THEN** the GI base ambient is 0.08 per channel

#### Scenario: High shading quality share
- **WHEN** the shading quality is 5
- **THEN** the light share is the single-precision float nearest 0.09

## ADDED Requirements

### Requirement: GI dynamic occupancy timing
The system SHALL change no occupancy while a block build is in flight, keeping the mesh's old frame. A block built after a mesh moved SHALL have that mesh moved from the frame its build used to the mesh's last frame. Each changed region SHALL be uploaded once into the block's occupancy texture.

#### Scenario: A mesh moves during a build
- **WHEN** a GI mesh moves while a block is being built
- **THEN** no occupancy changes until the build lands, and then the mesh is moved from its old frame

### Requirement: GI shell shape fragments
The system SHALL evaluate a registered shell or box GI shape per voxel with the original's fragment bodies: `gi_cuboid` the colour times `max(1 − d²·0.0002, 0)` with d the distance outside the half extents; `gi_dome` a 100-unit shell with `max(1 − d²·0.001, 0)`, a 70-unit cap and the ridge texture cubed times 6; `gi_bubble` a shell at its radius with the cubed texture, the vertical fade cut and 4×; `gi_telebubble` the three-plane texture, pulse and doubled square.

#### Scenario: Cuboid edge
- **WHEN** a cuboid shape of half extent 1 is sampled 0.5 units outside one face
- **THEN** its light is the colour times 0.99995

### Requirement: GI line and point shape fragments
The system SHALL evaluate `gi_gradline` as its two-ended gradient times 3 plus its point light, and `gi_multipoint` as each point's colour times `max(1 − d²·Atten, 0)²·0.25`, leaving the occlusion term unapplied. A shape entity SHALL be listed while it carries its shape and unlisted when it loses it.

#### Scenario: Multipoint at a point
- **WHEN** a multipoint shape's red point sits on the voxel
- **THEN** the voxel takes 0.25 red from it

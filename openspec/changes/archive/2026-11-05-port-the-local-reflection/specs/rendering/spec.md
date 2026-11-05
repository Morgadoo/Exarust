# Spec Delta

## ADDED Requirements

### Requirement: Local reflection
A surface whose material has the original's local reflection (a reflection layer and extra flag `0x800` above shading quality 4) SHALL, inside the GI volume, mix its cube reflection toward a read of the previous frame's quarter-size blurred bright pass by `min(2·gloss, 1)²·(1 − N·V)`. The system SHALL draw no mirrored view of the scene.

#### Scenario: A lamp globe's rim
- **WHEN** the dark glass of a lamp globe is seen inside the GI volume
- **THEN** its rim, seen edge on, shows the blurred glow of the previous frame instead of the cube, and its face keeps the cube reflection

### Requirement: Local reflection read
The local reflection's read SHALL sit at the fragment's screen position pushed by `n.xy · (1.05/(1.05 − n.z) − 1) · 0.05 · ViewPrj / w`, with `n` the view-space normal, `w` the clip depth in centimetres and `ViewPrj` the projection's scales × 500, and SHALL be scaled by 1.5.

#### Scenario: A normal facing the eye
- **WHEN** a fragment's view-space normal points at the eye
- **THEN** the read is not pushed

### Requirement: Supersampled sub-frame view
With supersampling on, the debug variable `EXA_SUBFRAME` set to 0–3 SHALL show that sub-frame alone (the pixel of each window pixel's 2 × 2 block at column `k & 1`, row `k >> 1`) instead of the resolve.

#### Scenario: One sub-frame
- **WHEN** `EXA_SUBFRAME=3` is set
- **THEN** edges show one sample's steps and the sub-frame's own shadow taps

## MODIFIED Requirements

### Requirement: GI per-object block pick
The system SHALL light each object from the resident GI block its world bounds centre falls in (the original's node bounding-sphere centre): the centre less the block grid's low edge, in centimetres, times 1/4800, floored on each ground axis, names the block; terrain SHALL use the centre of each 6 m terrain block and plants their block's centre. Surfaces SHALL sample the picked block with no box test, and an object whose block is not resident SHALL take no GI light.

#### Scenario: Ground across a block edge
- **WHEN** the camera's block ends across a shadowed floor and the block beyond it is resident
- **THEN** the floor beyond the edge takes that block's GI light and shows no step at the edge

#### Scenario: No resident block
- **WHEN** an object stands in a block that holds no light texture
- **THEN** its GI ambient is black, as with the original's texture 0

#### Scenario: A mesh away from its origin
- **WHEN** a wall's node origin lies in one block and its geometry's centre in the next
- **THEN** the wall takes the next block's light

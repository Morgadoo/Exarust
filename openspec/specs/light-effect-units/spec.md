# light-effect-units Specification

## Purpose
Keeps the rules of the original's light, shadow, material, spline and
particle units as tested data and plain code: the spline strip, the particle
UV table, the shadow-map sizing, the material defaults and the voxel light
fields. Most of them are not driven by the running renderer yet.

## Requirements

### Requirement: Spline strip
The strip SHALL pre-build an index list for 2049 quads of two triangles each, add two vertices per point either side of the current point along the chord normal scaled to the draw-state width, store the join curvature in double precision, and zero the first vertex's curvature in mode 2 and the second's in mode 1. Closing without a point SHALL extend the previous point through the current one to twice the distance and drop the final pair from the draw.

#### Scenario: Close without a point
- **WHEN** a strip ends with no explicit point
- **THEN** its last real point gets the normal of a straight continuation and the draw holds two vertices fewer than were written

#### Scenario: Fade
- **WHEN** the fade is set
- **THEN** each pair's alpha is the colour alpha times its first vertex index over the segment count times one half, rounded to even, low byte only

### Requirement: Particle UV table
The 128-entry table SHALL hold four rows of eight symmetries of four corners, each corner halved and moved to the row's atlas origin.

#### Scenario: Matches the quad builder
- **WHEN** a particle uses the 32-variant atlas
- **THEN** its corners equal the table entries for its cell and symmetry

### Requirement: Shadow-map sizing
The allocator and the cube map's own draw SHALL size a map separately: the allocator by class, quality index, reduction shift and signed hardware cap, the draw by the user detail plus the light offset with no shift and no cap.

#### Scenario: Cube sizes
- **WHEN** a cube light has quality index 3
- **THEN** the allocator's size is the cube table entry for 3 halved, then shifted by the reduction level

### Requirement: Material defaults
An unknown material name SHALL shade with one shared built-in `_Default` record rather than a fresh default each time.

#### Scenario: Two unknown names
- **WHEN** two models name materials the loader does not know
- **THEN** both use the same `_Default` handle

### Requirement: Base light unchanged
Voxel light data SHALL add radii and a flag word without changing the intensity the existing GI injects.

#### Scenario: Spawned node
- **WHEN** a model node with a voxel light is spawned
- **THEN** its current and base intensity both equal the record's

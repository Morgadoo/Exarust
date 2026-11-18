## ADDED Requirements

### Requirement: Fixture glow put-out
A fire record's load set-up SHALL put out a fixture's glow by zeroing the emissive of a mesh node's first part and of each mesh child's first part, writing no light; by day the target SHALL be the mesh the first fire marker under the object hangs from, the marker when its parent is not a mesh, or the object without a marker.

#### Scenario: A fixture by day
- **WHEN** a fire record's object is set up while the daylight is above the threshold
- **THEN** the first part of the marker's host mesh and of each of its mesh children loses its glow and no fire is lit

### Requirement: Level-load switch-off
An operative record with flag `0x40` SHALL darken every part of its node and each mesh child's first part, and SHALL clear each voxel-light child's current intensity without re-deriving the light's colour.

#### Scenario: A switched-off voxel light
- **WHEN** a flagged record's node has a voxel-light child
- **THEN** that light's intensity is zero and its injected colour stays the one it last derived

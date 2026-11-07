# Spec Delta

## MODIFIED Requirements

### Requirement: Cut-away fade
The system SHALL move a drawn or seen mesh's fade as the original does: not cut away, an amount above 0 drops by 0.003 per millisecond and ends at 0 at or below 0.001 (double precision), otherwise the mesh is drawn fading; cut away, the amount rises by 0.003 per millisecond, below the target less 1.19209e−7 a drawn mesh is fading, otherwise the amount snaps to the target and below 0.999 (double precision) a drawn mesh is settled, else not drawn.

#### Scenario: Out and back
- **WHEN** a drawn mesh with target 1 is cut away for 100 ms, then 300 ms
- **THEN** its amount is 0.3 and it is fading, then the amount is 1 and it is not drawn

#### Scenario: Thresholds
- **WHEN** a fading-in mesh holds exactly 0.001 for a frame of 0 ms
- **THEN** it stays fading at 0.001
- **WHEN** a cut-away mesh holds exactly its target less the epsilon
- **THEN** it snaps to the target

### Requirement: Cut-away probe
The probe SHALL test only the sectors the first view draws and only meshes whose scene word carries the cut-away candidate bit, with the segments taken into the sector's space. An overhead mesh SHALL be tested with a 10 m segment straight down onto the looked-at point, the others with the camera's segment, against the bounding sphere. A hit SHALL then mark a mesh as its kind's callback answers; any other kind SHALL be cleared; a miss SHALL clear a marked mesh.

#### Scenario: On the line and overhead
- **WHEN** candidate meshes stand on the camera's segment, 10 m beside it, and 5 m over the player, in a drawn sector offset 100 m from the world origin
- **THEN** the mesh on the segment and the mesh over the player are marked and the one beside it is not

#### Scenario: An undrawn sector
- **WHEN** the sector holding a marked mesh is not drawn by the first view
- **THEN** the probe leaves the mark as it is

## ADDED Requirements

### Requirement: Cut-away target
A mesh's fade target SHALL be 1 when its second flag word has `0x08000000` and ½ otherwise.

#### Scenario: The default target
- **WHEN** a candidate without `0x08000000` is cut away for 300 ms
- **THEN** its amount settles at ½ and it is drawn settled

### Requirement: Cut-away callbacks
A sphere-kind mesh SHALL be marked on the sphere alone. A segment-kind mesh SHALL be marked when a 50 cm sphere swept along the segment in the mesh's space meets one of its triangles from the front; an overhead mesh when the downward ray meets one of its triangles from the front within 10 m. A mesh without triangles SHALL stop at the sphere.

#### Scenario: The faces decide
- **WHEN** the camera's segment passes through a segment-kind roof from above, and then a cast rises through it from below
- **THEN** the first marks the roof and the second, moving away from its faces, does not

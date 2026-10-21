# Spec Delta

## ADDED Requirements

### Requirement: Cut-away probe
The probe SHALL test only the sectors the first view draws and only meshes whose scene word carries the cut-away candidate bit, with the segments taken into the sector's space. An overhead mesh SHALL be tested with a 10 m segment straight down onto the looked-at point, the others with the camera's segment, against the bounding sphere. A hit SHALL mark a sphere-kind or callback-kind mesh and clear any other kind; a miss SHALL clear a marked mesh.

#### Scenario: On the line and overhead
- **WHEN** candidate meshes stand on the camera's segment, 10 m beside it, and 5 m over the player, in a drawn sector offset 100 m from the world origin
- **THEN** the mesh on the segment and the mesh over the player are marked and the one beside it is not

#### Scenario: An undrawn sector
- **WHEN** the sector holding a marked mesh is not drawn by the first view
- **THEN** the probe leaves the mark as it is

### Requirement: Cut-away carry
A mark or clear SHALL carry to the mesh's parent and to its mesh children when the mesh's bits say so, and a mesh that was not marked SHALL leave them alone.

#### Scenario: Parent and child
- **WHEN** a marked mesh with both carry bits is missed, and then a child is marked by hand under the unmarked mesh
- **THEN** the parent and child are cleared, and the hand-marked child keeps its mark

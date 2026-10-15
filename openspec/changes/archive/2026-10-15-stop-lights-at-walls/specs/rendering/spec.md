## ADDED Requirements

### Requirement: Lights stop at walls
In tile-grid levels the system SHALL flood each ranged light through the cell grid as the original does. It SHALL carry an angular window from cell to cell, narrowed by each side's opening and closed by solid walls, and SHALL record at most 12 lights per cell. It SHALL link the light to a mesh only if the mesh's angular extent overlaps the window in the mesh's cell or a neighbouring cell its box spills into.

#### Scenario: Light in the next room
- **WHEN** a light hangs in a room walled off from the camera's room
- **THEN** the meshes of the camera's room are neither lit nor shadowed by it

#### Scenario: Doorway
- **WHEN** a light shines through a doorway
- **THEN** only meshes within the beam through the door's opening are lit by it in the next room

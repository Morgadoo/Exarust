# Spec Delta

## ADDED Requirements

### Requirement: Merged cell floor
The system SHALL merge a tile-grid cell's floor (the floor set's pieces and the pieces' `+F` strips, placed in the cell's own space) into one `+Floor` node at the cell centre: one mesh per material run, in order of first use, pieces appended in placing order. `+`-material triangles SHALL take the planar floor UVs; other materials keep their UVs and tangents. The merged meshes SHALL be the cell's floor for the Ground Query, the GI floor points and the room view.

#### Scenario: A floored cell
- **WHEN** a dungeon cell with a floor set and walls on two sides is built
- **THEN** its floor is one `+Floor` node with one mesh per material, and the picture, the navigation grid and the GI floor points are those of the separate pieces

### Requirement: Room reset after the grid build
When the grid build has placed every cell, the system SHALL reset the room states as the original's builder does: every room hidden and none fading. The debug view of every room SHALL still show them all.

#### Scenario: A level finishes building
- **WHEN** a dungeon level's grid build ends
- **THEN** no room is active until the room anchor's update chooses its rooms

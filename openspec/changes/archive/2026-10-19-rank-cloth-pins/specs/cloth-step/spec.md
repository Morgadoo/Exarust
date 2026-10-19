# Spec Delta

## ADDED Requirements

### Requirement: Pin distance
Each vertex SHALL store its distance along triangle edges from the nearest pin. That distance SHALL follow the mesh when the straight line is not an edge. A pinned vertex SHALL have distance 0.

#### Scenario: Around the square
- **WHEN** a square of two triangles is pinned at one corner and the opposite corner shares no edge with it
- **THEN** the opposite corner is two edge lengths away, not the straight-line length

### Requirement: Draw hook
A double-sided cloth SHALL draw in two passes, culling the front and then the back. A single-sided cloth SHALL draw only with the back culled.

#### Scenario: Both faces
- **WHEN** a mesh cloth is asked for its passes, then marked single sided
- **THEN** the first list culls the front and the back, and the second list culls only the back

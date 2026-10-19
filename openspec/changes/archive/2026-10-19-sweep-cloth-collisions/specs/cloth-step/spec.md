# Spec Delta

## ADDED Requirements

### Requirement: Swept sphere
A body sphere whose path crosses a cloth triangle SHALL push a vertex on that crossing out past the triangle. A path that misses the triangle SHALL leave the vertex.

#### Scenario: Through and beside
- **WHEN** a sphere sweeps through a cloth vertex, then another sweeps beside the same triangle
- **THEN** the first vertex is 2 cm past the triangle and the second stays on it

### Requirement: Swept vertex
A vertex whose segment crosses a level triangle SHALL stop 2 cm short of the face it came from. A segment that misses the triangle SHALL keep its end.

#### Scenario: Onto the floor and past its edge
- **WHEN** a vertex sweeps down through a floor triangle, then another sweeps down beside it
- **THEN** the first stops 2 cm above the floor and the second keeps the end of its sweep

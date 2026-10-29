# Spec Delta

## ADDED Requirements

### Requirement: Prop collision mesh
A registered prop part SHALL carry its node mesh's triangles (metres, the node's frame) as its collision triangles; an index past the positions SHALL leave it none.

#### Scenario: A prop's mesh
- **WHEN** a mesh of positions (0,0,0), (1,0,0), (0,2,0), (0,0,3) with indices 0 1 2 0 2 3 1 belongs to a node with a registering body
- **THEN** its part carries the two triangles (0,0,0)(1,0,0)(0,2,0) and (0,0,0)(0,2,0)(0,0,3); with an index of 9 it carries none and the sphere fallback stays

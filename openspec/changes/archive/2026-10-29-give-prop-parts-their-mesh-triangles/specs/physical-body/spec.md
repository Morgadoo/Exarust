# Spec Delta

## MODIFIED Requirements

### Requirement: Mesh pair contacts
The more mobile part's spheres SHALL be tested against the other part's collision triangles in its frame: swept from the previous relative pose when they moved over 0.1 cm² (never in a pair with one placed body, and with no static-only gate), after touching the other's mesh box and mesh sphere, one triangle at a time with the sphere re-read after each accepted hit. A registered prop part SHALL carry its node mesh's triangles (metres, the node's frame) as its collision triangles. A part with no triangles SHALL be met through its spheres, pre-tested against their own bound.

#### Scenario: A fast sphere and a wall
- **WHEN** a sphere crosses a wall part's triangle at 60 m/s in one tick, with or without the static-only mode
- **THEN** the swept query catches it in front of the wall with one touch

#### Scenario: Behind the face
- **WHEN** a sphere's centre lies behind a mesh triangle
- **THEN** it is not pushed

#### Scenario: A prop's mesh
- **WHEN** a mesh of positions (0,0,0), (1,0,0), (0,2,0), (0,0,3) with indices 0 1 2 0 2 3 1 belongs to a node with a registering body
- **THEN** its part carries the two triangles (0,0,0)(1,0,0)(0,2,0) and (0,0,0)(0,2,0)(0,0,3); with an index of 9 it carries none and the sphere fallback stays

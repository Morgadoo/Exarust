## MODIFIED Requirements

### Requirement: Height Query
The level SHALL answer the original's Height Query: a sphere whose lowest point starts at a given point is swept straight down at most a given depth, each face taking the original's swept nearest test, and the answer is the height of its lowest point at the nearest hit. A face the sphere starts within its radius of is not met. Living bodies are not part of the level.

#### Scenario: Floor and step
- **WHEN** a 10 cm sphere drops from 70 cm onto a floor, and beside a 30 cm step whose edge is 5 cm from its centre line
- **THEN** it finds the floor, and 28.66 cm on the step's edge

#### Scenario: Faces turned away
- **WHEN** a face turned down lies over the floor, and when the sphere starts behind the floor
- **THEN** the first is passed through, and nothing is found

#### Scenario: Starting against the floor
- **WHEN** the sphere's lowest point starts 5 cm under a floor, its centre within its radius of it
- **THEN** that floor is not met

## ADDED Requirements

### Requirement: Mesh query closest point
The mesh query tests SHALL take a triangle's closest point by the face record's region test: the point dropped on the plane, tested against the two edges from the first corner, the far edge when a projection runs past the second corner or the face, each edge parameter clamped as the original's MAXSS and MINSS clamp it.

#### Scenario: Past a corner
- **WHEN** a point lies beyond a triangle's far corner on its plane
- **THEN** the closest point is that corner

### Requirement: Front sphere test floors
The front sphere test SHALL push a sphere whose centre is in front of a face within its radius and nearer than the radius less the floor to its closest point; under the floor, squared, it SHALL push along the normal. The floor SHALL be 1e-6 cm for a mesh with a kd-tree (more than 10 and fewer than 100 000 triangles over its node) and 1e-10 cm for one without.

#### Scenario: A centre on the face
- **WHEN** a sphere's centre lies 1e-4 cm over a face
- **THEN** a mesh with a tree pushes it along the normal, and one without pushes it off its closest point

### Requirement: Swept sphere test past the face
When a swept sphere's end reached a face and the face's closest point lies past the end within the radius, the push SHALL be the start's offset to that point scaled by the end's gap less the radius over the gap, lifted to the face when it would sink the end.

#### Scenario: Beside the edge
- **WHEN** a sphere sweeps down past a face's edge and ends just behind its plane, the edge point 5 cm from its end
- **THEN** the push is the start's offset to the edge point scaled by (5 − r)/5, lifted to the face

### Requirement: Mesh test unordered branches
The mesh query tests SHALL keep the original's unordered compares: a NaN plane distance goes past the front and swept tests' plane checks and the two-sided test's reach check.

#### Scenario: A NaN centre
- **WHEN** a sphere with a NaN centre is tested against a face
- **THEN** the front test reaches its push, a NaN

### Requirement: Overlap query mesh test
The "is the space free" query SHALL test a part's spheres against another part's mesh two-sided: a face whose plane is within the radius on either side and whose closest point is nearer than r² less the engine's small float touches.

#### Scenario: Behind a face
- **WHEN** a probe sphere lies 1 cm behind a face of another body's mesh, within its radius
- **THEN** the space is taken

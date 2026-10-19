# cloth-step Specification

## Purpose
One cloth tick: how a particle falls, how wind and a pin move it, and how an edge corrects.

## Requirements

### Requirement: Fall
A particle at rest SHALL drop by the body's gravity for one tick, in centimetres. Its previous position SHALL be where it was.

#### Scenario: Resting vertex
- **WHEN** a particle at rest is stepped with the cloth gravity
- **THEN** its height falls by that gravity and its previous height is unchanged

### Requirement: Edge
An edge at its rest length SHALL not move its ends. A stretched edge SHALL pull its ends closer.

#### Scenario: Rest and stretch
- **WHEN** an edge of length 20 joins particles 20 apart, then one of length 10 joins particles 20 apart
- **THEN** the first pair stays put and the second pair is closer than 20

### Requirement: Pin and wind
A pin of weight 1 SHALL place its particle on the rest point. Wind above 0.001 SHALL move a particle by direction times (noise + 0.25) times the wind. Wind at 0.001 SHALL not.

#### Scenario: Held and blown
- **WHEN** a pin of weight 1 has its rest 5 cm along X, then a particle is blown by wind 1 along X with noise 0, then by wind 0.001
- **THEN** the pinned particle sits at 5 cm, the blown one moves 0.25 cm, and the last one stays

### Requirement: Sphere and plane
A vertex inside a body sphere SHALL be pushed out to the radius plus 2 cm. A higher friction SHALL leave less tangential motion. A vertex behind a level plane SHALL be pushed out to 2 cm along the outward normal.

#### Scenario: Clearance
- **WHEN** a vertex at a sphere's centre of radius 10 cm is stepped, then one behind a horizontal plane
- **THEN** the first ends 12 cm from the centre and the second sits 2 cm above the plane

### Requirement: Sleep
A still cloth SHALL count its sleep down, and SHALL NOT go below its owner's count. At zero it SHALL NOT fall. A push SHALL wake it.

#### Scenario: Still, then pushed
- **WHEN** a still cloth counts down to sleep, then a sphere covers it
- **THEN** it does not fall while asleep, and the push moves it and wakes it

### Requirement: Rustle and mesh
Rustle SHALL play when the squared sum of the velocities exceeds 400, and its envelope SHALL decay by 0.9 when the cloth is quiet. The mesh vertices SHALL be the particle positions.

#### Scenario: Fast cloth
- **WHEN** a fast cloth is stepped, then a quiet one, and a falling vertex is read as a mesh
- **THEN** rustle plays and then decays, and the mesh matches the moved vertex

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

### Requirement: Mesh
A triangle mesh SHALL make one particle per vertex, one link per triangle edge, and one bend link across each shared edge. A pinned vertex SHALL be held at its point. The face list SHALL contain both windings.

#### Scenario: Two triangles
- **WHEN** two triangles share an edge and one vertex is pinned
- **THEN** there are six links including the bend across the shared edge, the pin weight is 1, and both windings are listed

### Requirement: Wind tile
Scene wind SHALL be scaled by 0.3. Each tile sample inside −1 to 1 SHALL walk by `(random − 0.5)` times that wind times 0.8. A sample of 1 everywhere SHALL blow a particle by 1.25 times the wind along the wind direction when the wind strength is 1.

#### Scenario: Flat tile
- **WHEN** every sample starts at 0 and the random value is 1 under wind 1, then a particle is blown by a tile of all 1s
- **THEN** the sample is 0.4 and the particle moves 1.25 cm along the wind

### Requirement: Body reaction
The reaction SHALL be the opposite of the sum of this tick's collision pushes.

#### Scenario: One push
- **WHEN** a sphere pushes the only vertex
- **THEN** the reaction is the opposite of that vertex's move

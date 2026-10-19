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

### Requirement: Draw submission
A double-sided cloth's submitted passes SHALL become two draw materials, the first culling the front and the second the back. A single-sided cloth SHALL become one material that culls the back.

#### Scenario: Front then back
- **WHEN** a mesh cloth is built and its draw materials are taken from a plain surface, then the cloth is marked single sided and the materials are taken again
- **THEN** the first pair culls the front and then the back, and the second list is one material that culls the back

### Requirement: Cloth start
Start SHALL add 0x10000 to the flags, copy the seed, install a 4×4 identity matrix and set the step to the square root of the count times 1e-6. A body whose flags carry 0x10000000 SHALL set the link bit and be stored.

#### Scenario: The start
- **WHEN** the count is 10000, the seed is 7 and the body carries 0x10000000
- **THEN** the flags gain 0x10000 and the seed is 7
- **AND** the matrix is the identity
- **AND** the step is 0.1
- **AND** the link bit is set and the body is stored
- **WHEN** the body lacks the bit
- **THEN** the link bit is clear and no body is stored

### Requirement: Cloth frame swap
A pending count of at most one SHALL copy the source into both spare buffers twice, clear the count and set flag 1. A larger count SHALL rotate the three buffers and store the frame index.

#### Scenario: The pending copy
- **WHEN** the pending count is 1 and the frame is 9
- **THEN** two copies run, the count becomes 0, flag 1 is set and no frame index is stored
- **WHEN** the count is 2
- **THEN** three buffers rotate and the frame index is 9

### Requirement: Cloth activation
Activation SHALL OR the four flag bits, set phase 2 and scale 0.4, write the handle when present, copy the root origin and add 0x8000 when the slow global is set.

#### Scenario: The activation
- **WHEN** all four flag words are 0, a handle exists, the object is the root and the slow global is set
- **THEN** the words become 0x20000000, 0x10000, 0x204030 and 0x6000
- **AND** the phase is 2 and the scale is 0.4
- **AND** the handle and origin write once and the slow bit 0x8000 is added

### Requirement: Cloth transform
The transform SHALL make three calls per particle, then the object origin on flag 0x20 and the object extent always.

#### Scenario: Three particles
- **WHEN** the count is 3 and the object flags carry 0x20
- **THEN** nine particle calls, one origin call and one extent call run
- **WHEN** the flags lack 0x20
- **THEN** no origin call runs and the extent still does

### Requirement: Cloth rebuild
A non-root cloth SHALL return without work. A root SHALL copy its origin pointer. A present override that differs SHALL stop the rebuild. Otherwise the rebuild SHALL transform each particle point into a fresh buffer and submit it.

#### Scenario: The rebuild
- **WHEN** the cloth is its root, has no override and has 3 particles
- **THEN** the origin is copied, 3 points are transformed and the buffer is submitted
- **WHEN** the cloth is not its root
- **THEN** nothing is copied or submitted
- **WHEN** the override differs from the origin
- **THEN** nothing is transformed or submitted

### Requirement: Cloth sync
The sync SHALL skip its transform and pose when the object's meta flags carry 0x10. Otherwise it SHALL transform once and call the pose with 1.0 twice.

#### Scenario: The sync
- **WHEN** the meta flags lack 0x10
- **THEN** one transform and one pose call run with scale 1.0
- **WHEN** the meta flags carry 0x10
- **THEN** nothing runs

### Requirement: Cloth pressure
The pressure pass SHALL skip on gate 0x100. A normal under 1e-6 SHALL become zero with offset −1. Otherwise it SHALL normalize the normal, store the offset as its dot with the edge plus its length, remove the edge's normal component and scale or subtract it, then move the normal by the edge times the negative length. Afterwards it SHALL run coupled−1 calls, zero the normals and clear 0x1fe.

#### Scenario: One normal and one degenerate particle
- **WHEN** the gate is 0, the coupled word is 3, the first normal is zero and the second is (3,0,0) with edge (0,2,0) and e0 (0,2,5)
- **THEN** the first normal is zero with offset −1
- **AND** the second offset is 3 and its normal is about (1,−6,0.94868)
- **AND** its edge becomes about (0,2,−0.31623)
- **AND** two coupled calls run, the normals are zeroed and 0x1fe clears

#### Scenario: The gate
- **WHEN** the gate word carries 0x100 in its 0xf00 field
- **THEN** the pass returns without particle work or coupled calls

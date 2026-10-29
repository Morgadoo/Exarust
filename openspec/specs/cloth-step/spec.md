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
A vertex behind a level plane SHALL be pushed out to 2 cm along the outward normal. Body spheres SHALL push the cloth's triangles through the body stage and push pass (physical-body: Cloth body contact pushes triangles, Cloth push pass), not each vertex in turn.

#### Scenario: Clearance
- **WHEN** a vertex behind a horizontal plane is stepped
- **THEN** it sits 2 cm above the plane

### Requirement: Sleep
A still cloth SHALL count its sleep down, and SHALL NOT go below its owner's count. Asleep, it SHALL NOT fall. A body that pushes it SHALL wake it only when the body's countdown is higher than the cloth's.

#### Scenario: Still, then pushed
- **WHEN** a still cloth counts down to sleep, then a moving body's sphere covers it
- **THEN** it does not fall while asleep, and the push moves it and wakes it

### Requirement: Rustle and mesh
Rustle SHALL play when the squared sum of the velocities exceeds 400, and its envelope SHALL decay by 0.9 when the cloth is quiet. The mesh vertices SHALL be the particle positions.

#### Scenario: Fast cloth
- **WHEN** a fast cloth is stepped, then a quiet one, and a falling vertex is read as a mesh
- **THEN** rustle plays and then decays, and the mesh matches the moved vertex

### Requirement: Swept vertex
A vertex whose segment crosses a level triangle SHALL stop 2 cm short of the face it came from. A segment that misses the triangle SHALL keep its end.

#### Scenario: Onto the floor and past its edge
- **WHEN** a vertex sweeps down through a floor triangle, then another sweeps down beside it
- **THEN** the first stops 2 cm above the floor and the second keeps the end of its sweep

### Requirement: Mesh
A triangle mesh SHALL make one particle per position element (the vertices a seam splits SHALL share one particle), one link per triangle edge, and one bend link across each shared edge. A pinned vertex SHALL be held at its point. The face list SHALL contain both windings.

#### Scenario: Two triangles
- **WHEN** two triangles share an edge and one vertex is pinned
- **THEN** there are six links including the bend across the shared edge, the pin weight is 1, and both windings are listed

#### Scenario: A seam
- **WHEN** a square's two triangles are drawn with two of its four corners split across a UV seam, six vertices in all
- **THEN** the cloth has four particles and six links, and both copies of a split corner draw the same particle

### Requirement: Wind tile
Scene wind SHALL be scaled by 0.3. Each tile sample inside −1 to 1 SHALL walk by `(random − 0.5)` times that wind times 0.8. A sample of 1 everywhere SHALL blow a particle by 1.25 times the wind along the wind direction when the wind strength is 1.

#### Scenario: Flat tile
- **WHEN** every sample starts at 0 and the random value is 1 under wind 1, then a particle is blown by a tile of all 1s
- **THEN** the sample is 0.4 and the particle moves 1.25 cm along the wind

### Requirement: Body reaction
The reaction SHALL be the opposite of the sum of the vertices' moves from this tick's collisions and push passes.

#### Scenario: One push
- **WHEN** a moving body's sphere wakes a sleeping cloth and pushes its triangle
- **THEN** the reaction is the opposite of the vertices' summed move

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

### Requirement: Cloth destroy
The destructor SHALL make one release and one final call on a positive free flag, free each of four stored pointers present, free the origin only for a root, destroy three managed fields, and free each list entry and the list when present. The base call SHALL always run.

#### Scenario: The freeing path
- **WHEN** the free flag is set, all five pointers are present, the cloth is its root and the list holds 3 entries
- **THEN** one release, five frees, three field calls, 3 list frees and one list free run
- **AND** the base and final calls each run once

#### Scenario: The quiet path
- **WHEN** the free flag is clear and no pointer or list is present
- **THEN** only the three field calls and the base call run

### Requirement: Cloth body collision
A cloth tested against its own body SHALL return unchanged. An accepted hit SHALL make one callback, store its index and side, and, when resolved, SHALL snap the particle to e0 inside the radius or place it at the radius along the edge. Other particles SHALL be unchanged.

#### Scenario: The push-out
- **WHEN** a particle at (0,2,0) hits e0 (0,0,0) with radius 1 and resolves
- **THEN** it becomes (0,−1,0) and one callback runs
- **WHEN** the particle is at (0.5,0,0)
- **THEN** it becomes (0,0,0)

#### Scenario: The rejections
- **WHEN** the body is the cloth's own
- **THEN** nothing runs
- **WHEN** a particle misses, fails a segment test or does not resolve
- **THEN** it is unchanged and the last case still counts its callback

### Requirement: Cloth clone
The clone SHALL create one new cloth and copy fields 0x20, 0x140, 0x144 and 0x148 unchanged.

#### Scenario: The copy
- **WHEN** the fields are 0x1122, 0x33, 0x44 and 0x55
- **THEN** the new cloth is created and holds 0x1122, 0x33, 0x44 and 0x55

### Requirement: Cloth owns its mesh
The system SHALL give each cloth node a private copy of its mesh asset before the cloth moves its vertices, and SHALL skip for good a cloth node whose mesh has no positions or no indices.

#### Scenario: Two copies of one model
- **WHEN** two entities share one mesh asset and both carry a cloth marker
- **THEN** each holds its own mesh asset and the cloth of one does not move the vertices of the other

### Requirement: Cloth importer gate
The system SHALL give a marked node a cloth controller only when the scene the node is loaded into has the cloth scene flag (0x10000).

#### Scenario: Marker without the flag
- **WHEN** a node carries the cloth marker in a scene without the flag
- **THEN** it gets no controller

### Requirement: Cloth pin list
A cloth SHALL pin a position element only when every face about it carries a bit of `0xF0000000` in the mesh's face flags. The pin weight SHALL be 1 when each of those faces also carries a bit of `0xC0000000`, and 0.1 otherwise. An element with any face lacking the bits SHALL hang free.

#### Scenario: A marked hem
- **WHEN** a square's two triangles share an edge, the first is flagged `0x80000000` and the second 0
- **THEN** only the first triangle's corner off the shared edge is pinned, at weight 1

#### Scenario: A loose mark
- **WHEN** the second triangle is flagged `0x10000000` instead
- **THEN** every corner is pinned: the first triangle's free corner at 1, the others at 0.1

#### Scenario: A banner at rest against sleeping props
- **WHEN** a banner pinned by its hem hangs against two sleeping body spheres that cut into its rest pose
- **THEN** it falls asleep, and its hem stays within 1 cm of its rest

### Requirement: Placed cloth start
A cloth whose mesh a level placement record replaces SHALL take the record's positions from the level's frame into its node's own frame. It SHALL start with the record's word as its sleep countdown, asleep when the word is under 1. A cloth without such a record SHALL start awake at 32.

#### Scenario: A placed banner
- **WHEN** a banner is placed turned half round, and its record stores a point 10 cm in front of the placement with the word 0
- **THEN** the point sits 10 cm along the node's own z, and the cloth starts asleep

### Requirement: Cloth parameter record
A cloth SHALL be set up from a parameter record: the built-in record (mode `0x7208`, slide 0.1 cm, retention 0.5 a second, drag 0.05, padding 2 cm, gravity 1, four passes) unless its node hands it another. The per-tick retention SHALL be 0.98977 times `exp(ln(r) · 1/83)` computed in double and stored as a float.

#### Scenario: The built-in record
- **WHEN** a banner's cloth is set up
- **THEN** it keeps 0.98977 · 0.5^(1/83) of its velocity a tick, drags 0.05 toward the mean, and meets the level with 2 cm padding and a 0.1 cm slide

### Requirement: Constraint mode
A record whose mode word has `0x100` in its `0xF00` nibble SHALL make all its passes in the step and none after the body push; any other SHALL make one in the step and the rest after the push.

#### Scenario: The hair's record
- **WHEN** a cloth is set up from the hair's record (mode `0x104`, drag 0.2, padding 0, slide 0, two passes)
- **THEN** its step makes two constraint passes, its post-body pass makes none, and its level contact uses a zero radius and no slide

### Requirement: Particle-mode body contact
A particle-mode cloth SHALL take each body's spheres in order, padded 1 cm (none on its owner), skip a sphere apart from the cloth's box (tested with the squared padded radius as its radius), and push each particle inside the padded sphere straight out to its surface when the push's squared length is at least 1e-4 by bit pattern. A body that pushed SHALL hand the cloth its countdown when that is higher.

#### Scenario: A particle inside a sphere
- **WHEN** a still particle sits 5 cm from the centre of a 10 cm sphere of another body
- **THEN** it moves out to 11 cm from the centre

### Requirement: Particle-mode friction
A pushed particle's travel over the tick, less the body's motion at it (tetrahedron weights with the approximate reciprocal), SHALL lose its part along the push; the part across it SHALL be cancelled when its square is within a tenth of the push's square (and over the engine's small float), else shortened by `√(0.1·push²/travel²)` of itself.

#### Scenario: A particle sliding over the scalp
- **WHEN** a particle 5 cm from an owner sphere's centre of radius 10 cm travelled 3 cm across the push
- **THEN** it moves out to the surface and keeps 3·(1 − √(2.5/9)) cm of that travel

### Requirement: Owned cloth
A particle-mode cloth SHALL never sleep below its owner's countdown, the owner being the body its node's parent carries, and its particles SHALL keep their world motion when the owner moves the node while its pins follow the node.

#### Scenario: Hair on a living character
- **WHEN** a hair cloth hangs from the head of a character whose body is awake (countdown 256)
- **THEN** the cloth stays awake; when the body sleeps, the cloth may sleep

### Requirement: Bodies offered to a cloth
The cloth stage SHALL offer a cloth only the body parts whose mesh box's world centre lies in the nine 600 cm grid cells around the cloth's own cell (its own cell, then the eight around it), before the sleep filter and the object pre-test; a part three cells away SHALL never be offered, however many there are. The offered parts SHALL be found from a bucketing of the level's parts built once a tick, not by scanning every part for each cloth.

#### Scenario: Near and far parts
- **WHEN** an awake cloth hangs in cell x 0 with one body part in its cell, one in the next cell and twenty three cells away
- **THEN** two parts are offered to it

#### Scenario: A part across the cell edge
- **WHEN** a cloth's centre lies at x 599 cm and a part's box centre at x 601 cm
- **THEN** the part is offered

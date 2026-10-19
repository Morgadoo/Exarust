# Spec Delta

## ADDED Requirements

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

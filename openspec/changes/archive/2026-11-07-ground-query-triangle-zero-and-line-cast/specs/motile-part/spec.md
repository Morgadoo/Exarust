## ADDED Requirements

### Requirement: Mesh ray
The plain mesh ray SHALL walk a mesh's triangles in index order and take a triangle only when its determinant (the edge `v1 − v0` against `dir × (v2 − v0)`) is at least the engine's epsilon (front faces), both unnormalised barycentric terms lie in `[0, det]` with their sum at most `det`, and the distance lies strictly inside `(0, limit)`, the limit shrinking to each hit taken. The dot products SHALL sum x, then y, then z. It SHALL return the index and distance of the last triangle taken.

#### Scenario: Two faces at one distance
- **WHEN** two triangles at the same distance are met
- **THEN** the one earlier in index order is returned

#### Scenario: Turned away
- **WHEN** the ray meets the back of a face
- **THEN** the face is not taken

### Requirement: Ground Query triangle zero
The Ground Query SHALL run each floor piece's mesh ray within the nearest hit so far and SHALL drop a piece's hit when it is on triangle 0 of a piece that holds its node mesh's first triangle (the part built from the mesh group the index list starts with). A dropped hit SHALL not shorten the ray, and SHALL hide that piece's farther faces. The barricade's floor cast SHALL keep such a hit.

#### Scenario: Triangle 0 nearest
- **WHEN** a floor mesh's triangle 0 is the nearest face under the point and a farther face of the same mesh lies under it too
- **THEN** that mesh gives no hit

#### Scenario: Another mesh under it
- **WHEN** a separate floor mesh lies under the dropped triangle
- **THEN** the query returns that mesh's height

### Requirement: Line cast at the start of play
A new player at the level's own start SHALL be put at the point the line cast meets from the start lifted 10 m, straight down at most 30 m, against the front faces of every level mesh and rigid body stand-in (any triangle counting). A match layout's place, a landing plan and `--at` SHALL not be cast.

#### Scenario: A floor 2 m up
- **WHEN** the start lies under a floor 2 m up and over one at 0
- **THEN** the line cast meets the upper floor

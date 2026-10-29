## MODIFIED Requirements

### Requirement: Mesh
A triangle mesh SHALL make one particle per position element (the vertices a seam splits SHALL share one particle), one link per triangle edge, and one bend link across each shared edge. A pinned vertex SHALL be held at its point. The face list SHALL contain both windings.

#### Scenario: Two triangles
- **WHEN** two triangles share an edge and one vertex is pinned
- **THEN** there are six links including the bend across the shared edge, the pin weight is 1, and both windings are listed

#### Scenario: A seam
- **WHEN** a square's two triangles are drawn with two of its four corners split across a UV seam, six vertices in all
- **THEN** the cloth has four particles and six links, and both copies of a split corner draw the same particle

## ADDED Requirements

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

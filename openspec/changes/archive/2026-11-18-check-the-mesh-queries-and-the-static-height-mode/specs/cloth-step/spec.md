## ADDED Requirements

### Requirement: Cloth level contact on the mesh records
The cloth's level contact SHALL test each level triangle through its mesh record (the record builder's normal and plane and the record's closest point) and SHALL measure a reached sweep's gap with the sweep record's direction and length, rebuilt from the pushed end after each push.

#### Scenario: A sweep through a floor
- **WHEN** a 2 cm particle sweeps from 8 cm over a floor to 8 cm under it
- **THEN** it is pushed back to 2 cm over the floor

### Requirement: Degenerate level triangles skipped
The cloth's level contact SHALL skip a level triangle whose record normal is zero.

#### Scenario: A sliver triangle
- **WHEN** a level triangle has two equal corners
- **THEN** no particle is pushed or slid by it

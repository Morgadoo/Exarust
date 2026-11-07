## ADDED Requirements

### Requirement: Release wake and settling
When the item cursor lets go of an item (every release but the lock), the item SHALL stay awake at least 128 ticks (its sleep countdown raised only if lower) and settle 8 ticks with gravity and no inertia.

#### Scenario: A released item
- **WHEN** a lifted item is let go in reach
- **THEN** its countdown is at least 128 and it settles for 8 ticks

### Requirement: Placement overlap query at a release
At such a release the sector's placement overlap query SHALL run with the item at the frame the lift saved, over the body parts listed in the nine 600 cm cells around the item's mesh-box centre. For each part that passes the query's gates and whose pair test finds a touch (a collision sphere of the lower-key side, grown by 2 cm, meeting the other side's mesh), the lower of the two sleep countdowns SHALL be raised to the higher.

#### Scenario: A crate against the item's old place
- **WHEN** a sleeping crate's box lies 1 cm inside the released item's sphere bound at the saved frame
- **THEN** the crate's countdown is raised to the item's (128)

#### Scenario: A crate clear of the old place
- **WHEN** a crate's box lies 1 cm outside the item's sphere bound at the saved frame
- **THEN** its countdown is unchanged

### Requirement: Placement pre-test lets a NaN through
The placement pair test's two pre-tests SHALL skip a sphere only when the squared reach is below the distance², so a NaN distance goes on to the mesh query.

#### Scenario: A NaN distance
- **WHEN** a sphere's distance to the mesh centre is NaN
- **THEN** the pre-test passes it to the mesh query

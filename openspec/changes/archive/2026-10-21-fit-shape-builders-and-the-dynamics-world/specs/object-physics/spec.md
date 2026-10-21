# Spec Delta

## MODIFIED Requirements

### Requirement: Props registered as bodies
The system SHALL register every free or hung body tree (a top body and its body children) of a level's props, placed objects that are no door, lever or push surface, and own model: asleep (countdown 0) at its nodes' world matrices, at rest, its mobility the stored over its divisor, gravity by shares 4/mᵢ scaled by 4 over their sum over all four masses, the stored spheres against the level with friction 0.75. A blow SHALL wake it.

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** 4 713 trees of 6 023 bodies register asleep, 122 of them hung from the world; 198 mechanisms stay static

#### Scenario: Asleep where placed
- **WHEN** a registered prop 30 cm over the floor is ticked 500 times
- **THEN** its particles stay exactly where its node placed them and it stays asleep

#### Scenario: Shoved
- **WHEN** a prop 10 cm over the floor takes a 0.5 m/s blow
- **THEN** it wakes to 256, lands, slides about 6 cm and sleeps again 140 ticks later, resting on the floor

#### Scenario: Gravity at the centre of mass
- **WHEN** a body with point masses 2, 4, 8 and 8 is integrated once from rest
- **THEN** its particles fall 2, 1, 0.5 and 0.5 times 0.142352 cm

#### Scenario: Equal masses fall at g
- **WHEN** a body with four equal point masses is integrated once from rest
- **THEN** each particle falls 0.142352 cm

### Requirement: Builder resolution
The limb and root builders SHALL fit at 0.3 cells per cm when the mesh node's smallest box half extent is at least 10 cm and at 0.6 below it, with at least 6 cells along the longest axis, or at 1 cell per cm with at least 8 cells under their quality flag. A factory weapon SHALL fit at that resolution times its record's 1.1, with its record's even axes, 6 cells at least and no free corners, whatever the quality flag.

#### Scenario: A thin mesh
- **WHEN** a mesh node's smallest half extent is 9.99 cm
- **THEN** it is fitted at 0.6 cells per cm

#### Scenario: A factory weapon under the quality flag
- **WHEN** a weapon whose smallest half extent is 3 cm is built with the quality flag on
- **THEN** it is fitted at 0.66 cells per cm

## ADDED Requirements

### Requirement: Hung props
A registered tree whose top body carries stored links SHALL hang from the world: each link's parent point, plus the node's own translation, under its parent node's world matrix is the anchor its body point is held to (unshared); unless its first link is a slider, an origin pivot SHALL hold the body's origin at the node; the root SHALL skip level detection.

#### Scenario: A pinned crate
- **WHEN** a crate hung by one pin at its centre point is ticked 200 times
- **THEN** the pinned point stays within 1 cm of its anchor and the crate never falls out of the world

### Requirement: Shape builder tetrahedron
The shape builders SHALL lay a regular tetrahedron over the body's box with the axes ordered smallest, middle, largest: +√2 and −√2 on the smallest with −1 on the largest, ±√2 on the middle with +1 on the largest, the middle sign flipped for the odd orders; on a mirrored body whose box centre's x is over 5 cm every point's x and y SHALL be negated; all four points SHALL then be scaled by the half extents and moved to the box centre.

#### Scenario: Ascending axes
- **WHEN** a root of half extents 10 × 20 × 40 cm at the origin is built
- **THEN** its rest points are (14.1, 0, −40), (−14.1, 0, −40), (0, 28.3, 40) and (0, −28.3, 40) cm

### Requirement: Shape builder box and axes
A limb SHALL take its node's box cut so that it reaches the node's origin; a mirrored root or weapon whose box centre's x is over 5 cm from the centre plane SHALL grow its box over its children's. The two smaller extents SHALL be raised to a fifth of the largest (in double precision) and every extent to 6 cm. A mirrored body SHALL then take the axis order x, z, y when its origin's x is within 2 cm of the centre plane, else y, z, x when its box centre's x is over 5 cm from it.

#### Scenario: A mirrored root on the left
- **WHEN** a mirrored root at x = −20 cm with half extents 8 × 12 × 10 cm and a child of 9 × 1 × 16 cm is built
- **THEN** its box is 9 × 12 × 16 cm, its axes y, z, x, and its points are not flipped

### Requirement: Dynamics grid shift
A simulation's two 600 cm grids SHALL shift as the original moves its flat table: with the offset dy·w + dx, cell j takes what cell j − offset held and the cells left behind are emptied; a rightward shift then empties each walked row's first dx cells, a leftward one −dx cells from column w + dx − 1.

#### Scenario: One cell right
- **WHEN** a 3 × 2 grid whose cells hold 10 … 15 shifts by (1, 0)
- **THEN** its cells hold nothing, 10, 11, nothing, 13 and 14

### Requirement: Dynamics step driver
The step driver SHALL run the first simulation whose step's end is reached; one more than four steps past it and not paused SHALL first move its base so that the step ends now. When none is due it SHALL wait for the soonest end, at most 100 ms. A simulation's step SHALL update it (the pending grid shift, each sector's flags in order: dead, reset, scene left, shared scene, sub-scene, pending registration, then the asked-for rebuild) before its clock advances.

#### Scenario: A late simulation
- **WHEN** a 12 ms simulation whose base is 0 is checked at 1000 ms
- **THEN** it runs and its base becomes 988 ms

### Requirement: Placement overlap query
A placement SHALL be refused when the body or one of its child bodies meets another body in the nine body-grid cells around its bound sphere's cell (the cell itself first): only bodies whose bound sphere is not apart from the body's start bound, of another root, whose layers do not keep the two apart, and enabled are tested, the lower sector index first. A body whose cell word is 0 SHALL not be tested.

#### Scenario: Layers that keep bodies apart
- **WHEN** a body of layer 0x10 is placed beside one of layer 0x10 and one of layer 0x30
- **THEN** only the first is tested against it

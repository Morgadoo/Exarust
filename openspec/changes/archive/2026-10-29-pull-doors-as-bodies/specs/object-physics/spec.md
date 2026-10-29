## MODIFIED Requirements

### Requirement: Props registered as bodies
The system SHALL register every free or hung body tree (a top body and its body children) of a level's props, placed objects that are no lever, push surface or sliding door (a door's hinged leaves register, with their door rules), and own model: asleep (countdown 0) at its nodes' world matrices, at rest, its mobility the stored over its divisor, gravity by shares 4/mᵢ scaled by 4 over their sum over all four masses, the stored spheres against the level with friction 0.75. A blow SHALL wake it.

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** 4 822 trees of 6 146 bodies register asleep, 231 of them hung from the world (109 of those hinged door leaves); 89 mechanisms stay static

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

## ADDED Requirements

### Requirement: Door leaf body
A placed door's hinged leaf SHALL register as a hung body with mobility 0.05, the placement's word as its stored word, the handle at (−150·sign cm, the node box's centre height, 0), and a sibling node's leaf as its partner, both taking pair layer bits `0x3000`. It SHALL start shut when its stop reads within 0.04 rad of the shut angle.

#### Scenario: Pair layers
- **WHEN** a double door's two leaves register
- **THEN** both meet other bodies at mobility 0.05 and pass through each other

### Requirement: Door frame of a leaf
A leaf's door frame SHALL have the negated hinge axis as its up row, the stop's tied point in the parent node's frame (less the node's position) without its part along up, normalised, as its forward row, and up × forward, normalised, as its side row.

#### Scenario: A leaf at its file position
- **WHEN** a leaf whose stop link is taut on the shut side registers, its node turned by any yaw
- **THEN** it reads its shut angle (the half swing) and starts shut and passable

#### Scenario: A double door
- **WHEN** two leaves under one node and one under another register
- **THEN** the first two are each other's partner with layer bits `0x3000`, the third has none, and all sleep shut

### Requirement: Door leaf tick
Each tick an awake leaf SHALL measure the handle's motion since the last tick in cm, smoothed by half, as the creak energy, the stop's measured length as the knock travel, and the heading in the door frame of the stop point less the body's origin, turned into the parent node's frame, as the angle; run the door's step; and move the stop point to the point at the driven angle on the stop's radius about the body's origin, spread over the particles by its weights.

#### Scenario: Left alone
- **WHEN** a shut leaf is woken and ticked 1000 times
- **THEN** its angle stays within 0.04 rad of shut and it moves under 1 cm

#### Scenario: The first level's doors
- **WHEN** every one of `exanima01`'s 109 leaves is woken for two seconds of ticks
- **THEN** every shut leaf stays within 1 cm and shut, but for a double door's leaf whose partner stands open, and every open leaf stays inside its swing

### Requirement: Dense sphere sides
The pair stage's sphere fallback SHALL give the same hits, points and pushes when a part with 64 spheres or more has them bucketed by their centres as when every sphere is tested in order.

#### Scenario: A ball against a sheet
- **WHEN** a ball pushed into a 400-sphere sheet, still or swept, is collided with and without the grid
- **THEN** both give the same particles, hits and points

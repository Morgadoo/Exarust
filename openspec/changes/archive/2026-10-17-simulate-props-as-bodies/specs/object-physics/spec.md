## ADDED Requirements

### Requirement: Material mobility divisor
The system SHALL read a material record's physical divisor (parameter block `0xA4`, 0 read as 1) and impact sound prefix (`0xAC`, up to 4 characters; else the name's first 3), and give a rigid body the divisor of its node's first mesh part's material. It SHALL hold for every body node of `Objects.rpk` and `Objlib.rpk`.

#### Scenario: Shipped divisors
- **WHEN** the first material of every body node is resolved, its own record first, then the libraries'
- **THEN** `Objects.rpk`'s 1 230 bodies divide by 1 (1 144), 0.5, 2 and 3, and `Objlib.rpk`'s 1 503 by 1 (1 297), 0.6, 0.8, 2, 3 and 10

#### Scenario: Iron and wicker
- **WHEN** the models `anvil` and `basket_wkr a` are loaded
- **THEN** the anvil's body divides its mobility by 2 (`mtliron03`) and the basket's by 0.6 (`wkrtl01`)

#### Scenario: Impact sound set
- **WHEN** a record stores no prefix and its name is `mtliron04`
- **THEN** its impact sound set is `mtlcol`

### Requirement: Static props
The system SHALL treat a level prop placed with flag `0x80000`, and a model node marked with node flag `0x80000`, as level geometry: none of their rigid bodies registers and their meshes stay static colliders. It SHALL hold for every level's props and nodes.

#### Scenario: Shipped props
- **WHEN** the props of the 28 levels with props are read
- **THEN** 3 883 of the 79 935 are placed static, and none carries the frozen flag `0x800000`

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** the bodies of its 492 static props stay level geometry

#### Scenario: Arena tavern table
- **WHEN** `arenao1` loads
- **THEN** its own model's tavern table, a node marked `0x80000`, stays level geometry, and 17 bodies register

### Requirement: Props registered as bodies
The system SHALL register every free body tree (a top body without links and its body children) of a level's props, placed objects that are no door, lever or push surface, and own model: asleep (countdown 0) at its nodes' world matrices, at rest, its mobility the stored over its divisor, gravity by shares 4/mᵢ summing to 4, the stored spheres against the level with friction 0.75. A blow SHALL wake it.

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** 4 591 trees of 5 000 bodies register asleep; 122 trees hung from the world and 198 mechanisms stay static

#### Scenario: Asleep where placed
- **WHEN** a registered prop 30 cm over the floor is ticked 500 times
- **THEN** its particles stay exactly where its node placed them and it stays asleep

#### Scenario: Shoved
- **WHEN** a prop 10 cm over the floor takes a 0.5 m/s blow
- **THEN** it wakes to 256, lands, slides about 6 cm and sleeps again 140 ticks later, resting on the floor

#### Scenario: Gravity at the centre of mass
- **WHEN** a body with point masses 2, 4, 8 and 8 is integrated once from rest
- **THEN** its particles fall 2, 1, 0.5 and 0.5 times 0.142352 cm

### Requirement: Articulated props
The system SHALL join a body whose parent node has a body to the parent's body by its stored links (the parent point plus the node's local translation), then a pivot from its origin to that translation (slack 0, strength 1), each correction shared m/(m + m_p) and m_p/(m + m_p) by the two mobilities, with the limp body's link law. A child's slider link SHALL keep its stored value as plain slack.

#### Scenario: A sack's corner
- **WHEN** a 0.4676 child body is built on a 0.1370 body
- **THEN** its stored link and its pivot share corrections 0.7734 and 0.2266, and woken, the pair falls onto the floor with its pivot within 1 cm

#### Scenario: A child's slider
- **WHEN** a child body's link stores a 7 cm slider travel (49)
- **THEN** it becomes a link with 49 cm of slack

### Requirement: Props meet only the level
The system SHALL test a prop body's spheres against the level only, not against other bodies or their meshes, until bodies collide with each other. The meshes a body moves SHALL stay in the physics world as its stand-ins, moved with it, so character capsules and character bodies keep meeting it.

#### Scenario: Over another prop
- **WHEN** a prop body is woken 20 cm over another body's stand-in that lies 1 m over the ground
- **THEN** it falls through the stand-in to the ground, while a character's part lands on the stand-in

#### Scenario: Stand-in
- **WHEN** a body registered at the origin moves 1 m along x
- **THEN** its stand-ins move 1 m along x

### Requirement: Falling out of the world
The system SHALL remove a prop body, its nodes and its stand-ins once its root part's first particle is below −50 m, as the original removes the node.

#### Scenario: Nothing under it
- **WHEN** a body is woken with no level under it
- **THEN** it falls under 11.5 m/s, passes −50 m after 455 ticks and is removed

## MODIFIED Requirements

### Requirement: Part hit sound
A part's hit sound SHALL skip an armour layer only when its coverage is below 0.5, so a NaN coverage counts as covering. A character whose race byte has the golem bit SHALL ring HitGlm, and a bare part of the skeleton's race SHALL ring SkelHit.

#### Scenario: NaN coverage
- **WHEN** a layer of NaN coverage is the first to cover
- **THEN** the layer's material class sound is chosen

#### Scenario: A golem's limb
- **WHEN** a limb of a character of race byte 0x10 is struck
- **THEN** its hit sound is HitGlm

### Requirement: Attack reach
A character's step reach SHALL be size × ((1 − 0.3·feral) × base) in double precision, narrowed once, with base = (1 − 0.2·encumbrance) × 80 and size the single precision square root of the scale; a feral level below 0 SHALL use size × base, and a NaN level SHALL give a NaN reach.

#### Scenario: Human at feral level 0.5
- **WHEN** a human of scale 1 and no encumbrance has feral level 0.5
- **THEN** the reach is 1 × ((1 − 0.15) × 80) = 68 cm

#### Scenario: Scale 1.1 at level 0.84
- **WHEN** a character of scale 1.1, feral level 0.84 and no encumbrance is spawned
- **THEN** its step reach has the bits 1115359994, not those of 80·√scale rounded first

### Requirement: Held item drop words
A dropped weapon SHALL lose its strike, chain and mark bits (0x30001) over its whole tree, a dropped torch on its own body only, and a dropped shield none; every dropped item's own body SHALL take friction 0.75 and its base contact mobility, its children keeping theirs.

#### Scenario: Dropped torch
- **WHEN** a held torch with the held and mark bits is dropped
- **THEN** its mark bit clears and its held bit stays

#### Scenario: Dropped shield
- **WHEN** a held shield with the mark bit is dropped
- **THEN** its mark bit stays and its friction is 0.75

### Requirement: Torch friction
A held torch's own body SHALL have friction 0, while a held weapon's or shield's own body has 0.5; the item's other parts keep their friction.

#### Scenario: Torch in hand
- **WHEN** a torch is held
- **THEN** its body's friction is 0

#### Scenario: Two-part weapon
- **WHEN** a weapon of two parts is held
- **THEN** its top part's friction is 0.5 and its child's is what it was built with

### Requirement: Held item re-place after a move
When the game moves a character to a new place (the player's spawn placement on the ground, a Long Door, a golem's alarm-return pose reset, a roaming AI's revive placement), every item that character holds SHALL be put back at rest at its place under the holding hand where that hand now is, and woken. An item held flipped since its grip was built SHALL be turned half a turn about its own Y axis.

#### Scenario: A metre away
- **WHEN** a body holding a weapon is moved 1 m along x and 0.5 m along z and the re-place runs
- **THEN** the weapon's root part has moved by the same offset and its particles are at rest

#### Scenario: Flipped since built
- **WHEN** an item that was built unflipped is re-placed while held flipped
- **THEN** its place is the kept place turned half a turn about its Y axis, at the same point

#### Scenario: A golem sent home
- **WHEN** the alarm return sends a golem with a body home
- **THEN** its held items are asked to be re-placed, and a golem left awake asks for none

## ADDED Requirements

### Requirement: Golem foley
A golem body (a golem race, or a character wearing a golem suit) SHALL play FoleyPlH at volume 2 and base 1 when its race lacks the golem bit, Golem_Foley at 1 and 1 for race 0x12, and Golem_Foley at 6 and 1.5 for another golem race.

#### Scenario: A suit wearer's crowded impact
- **WHEN** a human wearing a golem suit takes a crowded limb impact of volume 0.5
- **THEN** FoleyPlH plays at 1.0

### Requirement: Golem layer class
At the end of every armour pass, a golem race's first armour layer covering at least half of each part (outer, then inner; a NaN coverage covers) SHALL take the golem class 6, so its wounds ring HitGlm.

#### Scenario: Outer layer under a half
- **WHEN** a golem's part has an outer layer of coverage 0.4 and an inner one of 0.9
- **THEN** the inner layer takes class 6 and the outer keeps its class

### Requirement: Weightless share
The armour pass SHALL give the character a weightless share of 0, then 1 or 0 for each pair of shield-lightening gauntlets worn, in wearing order, by whether it is powered; the holding pass SHALL lighten every held item by that share.

#### Scenario: Powered then unpowered gauntlets
- **WHEN** a character wears powered gauntlets and then unpowered ones
- **THEN** its weightless share is 0

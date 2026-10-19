# object-physics Specification

## Purpose
Props, items, doors, levers and pressure plates are rigid bodies in the
original, held by links and moved by the hand, by other bodies and by
signals. This capability covers their data (bodies, cloth markers, door and
operative records) and, as later changes land, their simulation.

## Requirements

### Requirement: Rigid bodies of models
The system SHALL decode the rigid body a mesh node carries as the original stores it: class word, mobility, the four rest points and point masses, the collision spheres, the links (points on the body and on the parent, slack, strength, smoothing, and a slider's axis, whose stored slack is its travel squared) and the scrape and roll sound slots, in metres. It SHALL hold for every chunk tree of `Objects.rpk`, `Objlib.rpk`, `Characters.rpk` and `Resource.rpk`.

#### Scenario: Shipped bodies
- **WHEN** every model of the installed `Objects.rpk` is decoded
- **THEN** 1 163 of the 1 253 models carry a body, all 1 230 bodies decode, 69 models carry links and 65 of them hang from the world

#### Scenario: A door
- **WHEN** the body of `door uw01a` is decoded
- **THEN** its mobility is 0.016, it has 1 899 spheres, a hinge pin 2.429 m up and a swing stop with 1.3404 m of slack

#### Scenario: Sliders
- **WHEN** the sliders of `pressplateu01` and `portcullis uwa` are decoded
- **THEN** the plate travels 0.04 m (stored 16) along its axis and the portcullis 1.70 m (stored 28 900), each equal to its axis's length

#### Scenario: Malformed body
- **WHEN** a body chunk is truncated or holds a count it cannot fit
- **THEN** decoding returns an error kept on the node, which still counts as carrying a body, and nothing panics

### Requirement: Cloth marker
The system SHALL report whether a mesh node carries the cloth marker, which makes the original simulate the mesh as cloth in the game's scene. It SHALL hold for every chunk tree of `CharExt.rpk`, `Objlib.rpk`, `Resource.rpk` and `Objects.rpk`.

#### Scenario: Shipped cloth
- **WHEN** the installed packs are decoded
- **THEN** the six long hair models carry the marker on two meshes each, four banners of `Objlib.rpk` and the wraith's robe on one, and no `Objects.rpk` model does

### Requirement: Door records
The system SHALL read an object's door section: its flags (latches, locked, not clickable, motorised, fast, inverted, frozen), the object it is linked to (its key, or the relay or lever that moves it) and the letters naming its lock, slam, hinge and unlatch sounds (`door`, `a` + letter, `_`, suffix). It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: Doors of the first level
- **WHEN** the database of `exanima01` is read
- **THEN** it holds 43 doors, whose linked objects are its keys, levers and relays but for one id it lacks

#### Scenario: A portcullis
- **WHEN** a door section with flags `0x254` and sound letter 5 is read
- **THEN** the door is locked, motorised and fast, and slams with `doorf_slm`

### Requirement: Operative records
The system SHALL read an object's operative section: class (key, lever, push surface, job, relay), kind, flags, value, a lever's return time in 100 ms steps and two targets. It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: Operatives of the first level
- **WHEN** the database of `exanima01` is read
- **THEN** it holds 72 operative records: 40 levers, 12 push surfaces, 6 fire jobs, 8 relays and 6 keys

#### Scenario: A returning lever
- **WHEN** a lever section with return time 50 is read
- **THEN** the lever returns after 5 000 ms

### Requirement: Mechanism of a model
The system SHALL give the body class the original gives a record's model: a lever for operative class 3, a push surface for class 5, none for any other operative class, else a door when the record has a door section. It SHALL hold for `objdb.rdb` and every level's own database.

#### Scenario: First level
- **WHEN** the records of `exanima01` are classified
- **THEN** 43 make doors, 40 levers and 12 push surfaces

#### Scenario: Operative class first
- **WHEN** a record holds a key's operative section and a door section
- **THEN** its model is no mechanism

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

### Requirement: Props meet bodies
The system SHALL test a prop body's spheres against the level and meet other bodies' parts (characters', corpses', other props') through the contacts between bodies, by its mobility, the group masks of its tree (a child never meets its parent) and no layer. The meshes a body moves SHALL stay in the physics world as its stand-ins, moved with it, for the character capsules; no body's spheres SHALL meet a stand-in.

#### Scenario: A cup on a table body
- **WHEN** a cup body (mobility 0.5) drops onto a sleeping table body (mobility 0.05) standing on the floor
- **THEN** it wakes the table, comes to rest on its top, and both sleep again with the table where it stood

#### Scenario: Pushed by a limb
- **WHEN** a character's limb thrown at 2 m/s meets a prop resting on the floor
- **THEN** the prop wakes and is pushed away

#### Scenario: Over a stand-in
- **WHEN** a prop body and a character's limb are woken over another body's stand-in lying 1 m over the ground
- **THEN** both fall through the stand-in to the ground

#### Scenario: Stand-in
- **WHEN** a body registered at the origin moves 1 m along x
- **THEN** its stand-ins move 1 m along x

### Requirement: Falling out of the world
The system SHALL remove a prop body, its nodes and its stand-ins once its root part's first particle is below −50 m, as the original removes the node.

#### Scenario: Nothing under it
- **WHEN** a body is woken with no level under it
- **THEN** it falls under 11.5 m/s, passes −50 m after 455 ticks and is removed

### Requirement: Hinged door
A hinged door's creak SHALL smooth the handle velocity by half and stop when the squared speed is at most 0.015. Otherwise the volume SHALL be 0.075 times the square root of the speed and the pitch SHALL be 0.022 times the speed plus 0.7, at most 1. A shut door's pull count SHALL rise by 1 up to 20 and then fall by 4 while it is held, not latched, and short of closed, and it SHALL unlatch when the count goes below 0. Opening SHALL shove the handle 3 cm along the door's facing.

#### Scenario: Creak and the seventh pull
- **WHEN** a still hinge is given a handle velocity of 1 along X, and a shut unlatched door whose count is 20 is pulled for seven ticks, and the door is opened along +Z
- **THEN** the creak pitch is 0.711, the counts after the first six pulls are 16, 13, 10, 7, 4 and 1, the seventh pull unlatches, and the shove is 3 cm along +Z

### Requirement: Door motor
A toggle or an open command SHALL aim at (1 − fraction) times the range times the sign, plus the closed angle. A plain step SHALL be the absolute error over the range, times 0.001, plus 0.0003. A motorised door SHALL step 0.0003, and a fast one 0.005. The drive SHALL move by that step toward the aim. A toggle SHALL arrive within its step, and an open command within 0.1 rad. A countdown below 2 SHALL end the command after this tick's drive.

#### Scenario: A plain step and an arrival
- **WHEN** a plain toggle aims 1 rad open from a shut angle of 0, a fast motorised toggle aims the same way, and an open command is 0.05 rad short of that aim
- **THEN** the drives are 0.0013 and 0.005 toward the aim, and the open command arrives on the aim and ends

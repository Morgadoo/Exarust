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

### Requirement: Sliding door
A sliding creak SHALL stop at energy 0.015 or below. Otherwise the volume SHALL be 0.2 times the square root of the energy and the pitch SHALL be 0.2 times the energy plus 0.7, at most 1. A motorised open SHALL drop the fraction by 0.01 a tick with smoothing 0.1 and stop below 0.01. A close SHALL add 0.075 with smoothing 0.05 and latch past 0.99. Shut and not held, the fraction SHALL stay 1 with smoothing 0.5. Held below 0.995, a pull of 4 SHALL unlatch on the seventh tick from 20.

#### Scenario: Creak, crawl, close, and the seventh pull
- **WHEN** the stored energy is 0.25, a motorised open at fraction 0.015 steps once, a close at fraction 0.95 steps once, a shut door is not held, and a shut door at count 20 and fraction 0.9 is held for seven ticks
- **THEN** the creak volume is 0.1 and the pitch is 0.75, the open fraction is 0 with smoothing 0.1, the close fraction is 1 and it is latched, the shut smoothing is 0.5, and the seventh pull unlatches

### Requirement: Sliding open
A plain open SHALL set the fraction to 0.6 for 80 ticks. A frozen door SHALL keep its stored fraction and stay passable. A hand point past 60 cm SHALL be pulled back to 60.

#### Scenario: A jump, a parked door, and the hand
- **WHEN** a plain open begins, a frozen door is parked at 0.4, and a hand offset is 100 cm along X
- **THEN** the fraction is 0.6 for 80 ticks, the parked fraction stays 0.4 and the door is passable, and the offset is 60 cm

### Requirement: Door partner
A single shut door SHALL stay shut at exactly 0.05 rad short of closed, and SHALL leave once it is farther and not latched, while still being held. A paired door whose partner is not within 0.1 rad SHALL not be held, and SHALL leave when its own angle is short of closed. A near partner SHALL keep it shut and held. Exactly 0.1 rad short SHALL not count as near, and past closed SHALL. A paired door SHALL add the pair bits to its layer.

#### Scenario: One leaf open, the other not held
- **WHEN** a single door is exactly 0.05 rad short and then 0.06 rad short and unlatched, a paired door 0.01 rad short has a partner that is not near, a paired door has a near partner, an angle is exactly 0.1 rad short and another is past closed, and a paired layer starts at 0
- **THEN** the single door stays shut on the line and leaves past it while still held, the open partner's leaf leaves the shut state and is not held, the near partner's leaf stays shut and held, exactly 0.1 rad short is not near, past closed is near, and the layer is the pair bits

### Requirement: Door unlock
An unlock SHALL clear the saved latch and start a wait of 1000 ms on the door and on its partner, while both stay latched. The partner SHALL keep its own saved latch. At the exact wait both SHALL stay latched. Once the clock is strictly past the wait, both latches SHALL open.

#### Scenario: A one second wait, then both leaves
- **WHEN** an unlock starts at time 0 on a latched door with a latched partner, the clock is then 1000, and then 1001
- **THEN** the saved latch is clear, the partner's saved latch remains, both stay latched through time 1000, and at time 1001 both latches are open

### Requirement: Lever step
A lever's creak SHALL smooth the arm velocity by half and stop at a squared speed of 0.015 or below. Otherwise the volume SHALL be 0.2 times the square root of that energy and the pitch SHALL be 0.2 times the energy plus 0.7, at most 1. The drive SHALL move 0.02 rad toward the target and SHALL hold once the gap is within 0.02. Held, the target SHALL be the hand. Forced, it SHALL be the off detent. Returning, it SHALL be the stored detent. Otherwise it SHALL be the nearest.

#### Scenario: Creak and a step toward the nearest detent
- **WHEN** a still arm is given a velocity of 1 along X at 0.2 rad from the first of two detents 1 rad apart, a return aims at the second detent from 0.5 rad, and a forced lever sits at 0.4 rad
- **THEN** the creak volume is 0.1 and the pitch is 0.75, the free drive is 0.18, the return drive is 0.52 toward 1, and the forced drive is 0.38 toward 0

### Requirement: Lever signal
Arriving more than 0.1 rad from the last detent SHALL report the rest flag at the first detent and its opposite at the second. A report of 0 SHALL signal 0 unless inverted, and SHALL stop a return. A report of 1 SHALL signal 1, or -1 when forced and inverted, and SHALL arm a timed return. A linked light SHALL fade out on 0 and in on 1. A report of 0 with a light SHALL not signal.

#### Scenario: An arrival signals, and a light fades
- **WHEN** a lever arrives at the first detent from a last detent of 1 with the rest flag clear, another arrives at the second detent with a return time, a held lever is forced and inverted on the first detent with the rest flag set, and a lit lever arrives at the first detent
- **THEN** the first signals 0, the second signals 1 and arms the return, the held lever signals -1, and the lit lever fades out without a signal

### Requirement: Plate travel
A pressed plate SHALL raise each travel by 0.02, at most 1, and a free plate SHALL lower each by 0.01, at least 0. A one-shot SHALL stop being pressed once a travel passes 1, and later travels that tick SHALL fall. The mean SHALL be written back to every travel. Grind speed SHALL be 0.9 times the mean's change plus 0.1 times the old speed, and SHALL stop at 0.001 or below. Otherwise volume SHALL be 50 times the speed, squared, and pitch SHALL be 20 times the speed plus 0.8, at most 1.

#### Scenario: A rise, a grind, and a one-shot
- **WHEN** a pressed plate starts at travel 0, and a one-shot plate with travels 0.99 and 0.5 is pressed
- **THEN** the first mean is 0.02 with grind volume 0.81 and pitch 1, and the one-shot is no longer pressed with both travels equal

### Requirement: Plate click
A mean above 0.7 SHALL press a plate that was up, click, and signal 1, or -1 when inverted. A mean below 0.5 SHALL release a plate that was down and signal 0, unless inverted. A travel of 0.68 that rises SHALL not press. A travel of 0.51 that falls SHALL not release.

#### Scenario: The click and the two edges
- **WHEN** a pressed travel of 0.68 steps, a pressed travel of 0.69 steps, a down plate falls from 0.51, and a down plate falls from 0.5
- **THEN** 0.68 stays up, 0.69 clicks and signals 1, 0.51 stays down, and 0.5 releases and signals 0

### Requirement: Signal door
A missing target SHALL do nothing. A door SHALL close on 0 and open on any other value, and -1 SHALL toggle: an unlatched leaf closes and a latched leaf opens. An inverted door SHALL flip that 0 and 1, and -1 SHALL still toggle. A door SHALL win over an operative record on the same object.

#### Scenario: Close, open, and a toggle
- **WHEN** a missing door is signalled 1, a door that also carries a fire job is signalled 0, then 1, then 2, then -1, and the same door inverted is signalled 0 and then 1
- **THEN** the missing door does nothing, the door closes, opens, opens, and toggles, an unlatched leaf closes, a latched leaf opens, and the inverted door opens and then closes

### Requirement: Signal job
A fire job SHALL start, at its stored size, only when the value is not 0. A light job SHALL set the stored bit to the value's low bit. A relay of kind 0, 1, 2, 3 or 5 SHALL be named, and any other kind SHALL be ignored. A plate SHALL be pushed exactly when the value is not 0.

#### Scenario: A fire, a light bit, a relay, and a plate
- **WHEN** a fire of size 200 is signalled 0 and then 1, a light whose stored word is 16 is signalled 1 and then 2, a relay of kind 4 is signalled, and a plate is signalled 0 and then -1
- **THEN** the fire does not start and then starts at size 200, the stored word becomes 17 and then 16, the relay is ignored, and the plate is free and then pushed

### Requirement: Relay combiner
A combiner SHALL set the source's bit when the value's low bit is 1 and clear it otherwise. A missing source SHALL leave it unchanged. When the state then equals the code it SHALL signal 1, or 0 when the invert flag is set. A mismatch SHALL signal the opposite, unless the silence flag is set, and the bit SHALL still be written.

#### Scenario: A bit, a match, and a silenced mismatch
- **WHEN** a missing source is applied, bit 0 is set against code 1, that bit is cleared, bit 1 is set while silenced against code 0, and bit 0 is set with the invert flag
- **THEN** the missing source leaves the state at 0, the match signals 1, the clear signals 0, the silenced state is 2 with no signal, and the inverted match signals 0

### Requirement: Relay mismatch
The second combiner SHALL write the same bit. A match SHALL signal 1 to its first target. A mismatch SHALL forward the original value to the source's second target when the state's low bit is clear, and SHALL do nothing when that bit is set.

#### Scenario: Match, forward, and a held low bit
- **WHEN** bit 0 is set against code 1, bit 1 is set against code 1 with the source's second target 12 and value 3, and the same bit is set while the state's low bit is already set
- **THEN** the match signals 1, the mismatch forwards 3 to target 12, and the held low bit sends nothing

### Requirement: Relay pair and sequence
A pair SHALL send the same value and source to both targets, and a missing pair SHALL send nothing. A sequence SHALL ignore a value of 0. A marked input SHALL count one step and store that mark in the high byte, and SHALL flag a wrong order when the mark overlaps the previous one. Eight steps without a wrong order SHALL signal the first target with 1 and reset. Fewer steps, or a wrong order, SHALL signal the second target with 1 and reset.

#### Scenario: Both targets, seven steps, and eight
- **WHEN** a missing pair is asked to forward, a pair forwards 5, a sequence is given value 0, seven distinct marks are finished, and eight distinct marks are finished
- **THEN** the missing pair sends nothing, both targets receive 5, value 0 changes nothing, seven steps signal the second target, and eight steps signal the first

### Requirement: Sequence listener
A finished sequence SHALL clear a listener whose class bits match and whose flags lack the skip bit, and SHALL leave any other listener.

#### Scenario: Who a finish clears
- **WHEN** a finish looks at a matching listener, one with the skip bit, and one whose class bits do not match
- **THEN** only the matching listener is cleared

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
A single shut door SHALL stay shut while its angle is no more than 0.05 rad short of closed, compared in double precision, and SHALL leave once it is farther and not latched, while still being held. A paired door whose partner is not within 0.1 rad SHALL not be held, and SHALL leave when its own angle is short of closed. A near partner SHALL keep it shut and held. Exactly 0.1 rad short SHALL not count as near, and past closed SHALL. A paired door SHALL add the pair bits to its layer.

#### Scenario: One leaf open, the other not held
- **WHEN** a single door with closed 1 is at 0.95000005 and then at the float nearest 0.95, another is 0.06 rad short and unlatched, a paired door 0.01 rad short has a partner that is not near, a paired door has a near partner, an angle is exactly 0.1 rad short and another is past closed, and a paired layer starts at 0
- **THEN** the single door stays shut at 0.95000005 and leaves at the float nearest 0.95 (it is past the double 0.05 line) while still held, the open partner's leaf leaves the shut state and is not held, the near partner's leaf stays shut and held, exactly 0.1 rad short is not near, past closed is near, and the layer is the pair bits

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
Arriving more than 0.1 rad from the last detent SHALL report the rest flag at the first detent and its opposite at the second. A report of 0 SHALL signal 0 unless inverted, SHALL stop a return and SHALL stop the tick loop. A report of 1 SHALL signal 1, or -1 when forced and inverted, and SHALL arm a timed return. A linked light SHALL fade in on 0 and out on 1, as detent 0 starts it at full. A report of 0 with a light SHALL not signal.

#### Scenario: An arrival signals, and a light fades
- **WHEN** a lever arrives at the first detent from a last detent of 1 with the rest flag clear, another arrives at the second detent with a return time, a held lever is forced and inverted on the first detent with the rest flag set, and a lit lever arrives at the first detent
- **THEN** the first signals 0, the second signals 1 and arms the return, the held lever signals -1, and the lit lever fades its light in without a signal

### Requirement: Plate travel
A pressed plate SHALL raise each travel by 0.02, at most 1, and a free one SHALL lower each by 0.01, at least 0, in double precision. A one-shot SHALL stop being pressed once a travel passes 1; later travels that tick SHALL fall. The mean SHALL be written back to every travel. Grind speed SHALL be 0.9 times the mean's change plus 0.1 times the old one, and SHALL stop at 0.001 or below, in double precision. Otherwise volume SHALL be (50 × speed)² and pitch 20 × speed + 0.8, at most 1.

#### Scenario: A rise, a grind, and a one-shot
- **WHEN** a pressed plate starts at travel 0, a still plate's old speed leaves it at the float nearest 0.001, and a one-shot plate with travels 0.99 and 0.5 is pressed
- **THEN** the first mean is 0.02 with grind volume 0.81 and pitch 1, the plate at the float nearest 0.001 still grinds, and the one-shot is no longer pressed with both travels equal

### Requirement: Plate links and wake
Every plate tick SHALL set each of the plate's links' smoothing to 0.1. A grinding plate SHALL keep its body awake for 256 ticks, and a quiet one SHALL not wake it.

#### Scenario: A grind wakes the body
- **WHEN** a pressed plate starts at travel 0, and a still plate at travel 0 steps
- **THEN** the pressed plate grinds and is awake, and the still plate is quiet and not woken

### Requirement: Plate click
A plate's state SHALL be the object word's low byte as stored. A mean above 0.7 SHALL press a plate whose state is 0, click, and signal 1, or -1 when inverted. A mean below 0.5 SHALL release a plate whose state is 1 and signal 0, unless inverted. Any other state SHALL neither press nor release. A travel of 0.68 that rises SHALL not press. A travel of 0.51 that falls SHALL not release.

#### Scenario: The click and the two edges
- **WHEN** a pressed travel of 0.68 steps, a pressed travel of 0.69 steps, a down plate falls from 0.51, a down plate falls from 0.5, and a plate whose state is 2 is pressed from 0.99 and then falls from 0
- **THEN** 0.68 stays up, 0.69 clicks and signals 1, 0.51 stays down, 0.5 releases and signals 0, and the plate in state 2 neither clicks nor signals and keeps state 2

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

### Requirement: Relay broadcast
A broadcast SHALL signal every tagged object whose door link equals the relay, including the source. A door link that does not match SHALL be skipped even when the operative target matches. Without a door link, an operative SHALL be signalled when its target equals the relay, except the source itself. An untagged id SHALL be skipped. The value and the relay id SHALL be forwarded in list order.

#### Scenario: The source door and one other operative
- **WHEN** the list holds an untagged listener, a source door linked to the relay, a door linked elsewhere that also listens, the source as an operative listener, another operative listener, and an operative listening elsewhere, and the value is 5
- **THEN** only the source door and the other operative are signalled, in that order, with the relay as the source and the value 5

### Requirement: Close and lock
A motorised hinge SHALL set its fraction to 1 and drive toward shut for 300 ticks, or 100 when fast. A hinge that is not motorised SHALL not start that drive. A sliding leaf that is not latched SHALL play the lock sound, and a fast one SHALL drive toward shut for 100 ticks. A leaf that is already shut SHALL become latched.

#### Scenario: A motor, a plain hinge, and a fast slider
- **WHEN** a motorised hinge at fraction 0.2 is closed, a fast motorised hinge is closed, a shut hinge that is not motorised is closed, an unlatched sliding leaf is closed, and a shut fast sliding leaf is closed
- **THEN** the motorised hinge is at fraction 1 for 300 ticks and is not latched, the fast hinge drives for 100 ticks, the plain hinge stays idle and becomes latched, the sliding leaf plays the lock sound and stays unlatched, and the fast slider drives for 100 ticks and becomes latched

### Requirement: Signalled open
A signalled open SHALL play the unlock sound once when there is at least one door, and SHALL play nothing when there are none. A latched or motorised door SHALL wait 120 ticks. A door that is neither SHALL not wait. While the countdown is at least 2 it SHALL fall by 1. Below 2 the wait SHALL end.

#### Scenario: One sound, then 120 ticks
- **WHEN** there are no doors, a door that is neither latched nor motorised is signalled, and a latched door is signalled and then stepped until the wait ends
- **THEN** the empty list plays nothing, the plain door plays the sound and does not wait, and the latched door ends its wait on the 120th tick

### Requirement: Open after the wait
When a signalled wait ends it SHALL open the door. The open SHALL clear the latch, the shut flag and the pull count. A hinge that is not motorised SHALL go idle and be shoved 3 cm along local Z. A motorised hinge SHALL set its fraction to 0 and drive open for 100 ticks, with the same shove. A sliding door that is not motorised SHALL jump to fraction 0.6 for 80 ticks and wake. A motorised sliding door SHALL drive open for 300 ticks, keep its fraction, and wake.

#### Scenario: The 120th tick opens
- **WHEN** a latched plain hinge, a latched motorised hinge at fraction 0.4, a latched plain slider, and a latched motorised slider at fraction 0.4 each finish a signalled wait
- **THEN** the plain hinge is idle, unlatched and shoved 3 cm, the motorised hinge is at fraction 0 for 100 ticks and shoved 3 cm, the plain slider is at 0.6 for 80 ticks and awake, and the motorised slider stays at 0.4 for 300 ticks and awake

### Requirement: Hinge toggle
A toggle SHALL open a latched door before it chooses. A hinge's openness SHALL be its angle minus closed, times -0.7. Above 0.7 the aim fraction SHALL be 1, and otherwise 0, including exactly 0.7. The hinge SHALL drive that aim for 100 ticks, or 60 when fast, and wake for 256 ticks.

#### Scenario: Shut opens, and 1.1 rad short closes
- **WHEN** a shut hinge is toggled, a hinge exactly 1 rad short of closed is toggled, a hinge 1.1 rad short is toggled, a fast shut hinge is toggled, and a latched shut hinge is toggled
- **THEN** the shut hinge aims at 0 for 100 ticks and wakes for 256, the hinge 1 rad short aims at 0, the hinge 1.1 rad short aims at 1, the fast hinge runs 60 ticks, and the latched hinge is unlatched and shoved 3 cm

### Requirement: Slider toggle
A slider's openness SHALL be 1 minus its travel. Above 0.7 it SHALL close for 60 ticks. At or below 0.7 it SHALL open for 60 ticks and clear the shut flag. A latched plain slider SHALL jump to 0.6 before that choice, so a shut one then opens for 60 ticks.

#### Scenario: An open slider closes, and a shut one opens
- **WHEN** a slider at travel 0 is toggled, a shut slider at travel 0.3 is toggled, and a latched plain slider at travel 1 is toggled
- **THEN** the open slider closes for 60 ticks and stays shut, the slider at 0.3 opens for 60 ticks and is no longer shut, and the latched slider is at 0.6 and opens for 60 ticks

### Requirement: Volume summary
The system SHALL summarise a body's voxel map as the original does: every filled cell from the low border to the last inner cell adds its index minus 0.5 to a running sum per axis, the centre is (sum/count/cells − 0.5) · half extent, and the mass is count · (2·h_z/n_z) · (2·h_y/n_y) · (2·h_x/n_x). A map with an axis of no cells SHALL have centre (0, 0, 0) and mass 0.1; a map with no filled cell SHALL have mass 0.

#### Scenario: A unit volume
- **WHEN** a 1 × 1 × 1 map over half extents (2, 4, 6) has its one cell filled
- **THEN** the centre is (0, 0, 0) and the mass is 384

#### Scenario: Two cells
- **WHEN** a 2 × 2 × 2 map over unit half extents has cells (1, 1, 1) and (1, 1, 2) filled
- **THEN** the mass is 2 and the centre is (−0.25, −0.25, 0)

#### Scenario: An empty volume
- **WHEN** a map has no cells along an axis
- **THEN** the centre is (0, 0, 0) and the mass is 0.1

### Requirement: Volume sort
The system SHALL sort entries by their float key with a middle-pivot two-way quicksort.

#### Scenario: Equal and out-of-order keys
- **WHEN** the keys are 3, 1, 2, 1
- **THEN** they are ordered 1, 1, 2, 3

### Requirement: Rigid-body voxel map
The system SHALL keep the original's voxel map for a rigid-body mesh: made at a resolution of 0.02 with a unit scale and the caller's tag, a box cut into cells with one empty border cell on each side, one word per cell running Z fastest, two fill bytes per cell and a list of extracted spheres. Releasing the map's arrays SHALL free the cells and fill bytes and keep the spheres.

#### Scenario: Border cells
- **WHEN** a map is sized to 2 × 3 × 4 cells inside its box
- **THEN** it holds 4 × 5 × 6 cells and its first inside cell's centre is half a cell in from the box's low corner

### Requirement: Door shape
A link of at most 30 cm of slack whose body point is farther than 1 cm SHALL replace a door's hinge axis, the first one always and later ones only when they do not point down; the axis SHALL start straight up. The last longer link SHALL be the swing stop, with radius r the length of its body point and half swing α twice the engine's fast arcsine of half the slack over r.

#### Scenario: The shipped stop
- **WHEN** a pin points 242.9 cm up and the stop's point is 164.8 cm to the negative side, 0.7 cm back, with 134.04 cm of slack
- **THEN** the radius is 164.80 cm, the half swing is 0.83806 rad and the axis points up

### Requirement: Door frame
A stop whose point lies on the positive side SHALL make a door's swing sign negative, and a later stop SHALL not set it back. The frame's zero SHALL be the stop's anchor, so the closed angle SHALL be the half swing α and the range SHALL be −2α times the sign.

#### Scenario: Closed and the range
- **WHEN** the shipped stop is on the negative side, and a second door has its stop 164.8 cm to the positive side
- **THEN** the first door's closed angle is 0.83806 rad and its range −1.67612, and the second door's sign is −1 with a range of 1.6761

### Requirement: Door stored word
A door's stored word SHALL be written only by its commands: the mode in its low byte and the countdown in its top byte, read back unsigned. A tick's countdown SHALL not be written back.

#### Scenario: A command writes the word
- **WHEN** a door is toggled for 100 ticks and then steps once
- **THEN** the stored word holds mode 8 and countdown 100

### Requirement: Lever shape
A lever's links SHALL decide its swing. The last link of at most 1 cm of slack whose body point is farther than 1 cm SHALL give the axle, negated. The last longer link SHALL be the arm: radius r and half swing α, twice the engine's fast arcsine of half the slack over r. The frame's zero SHALL be the arm's anchor and the two detents SHALL be at −α and +α.

#### Scenario: Two detents around the anchor
- **WHEN** a lever's arm link is tied at its anchor with a slack that gives a half swing α
- **THEN** its first detent is −α and its second is +α

### Requirement: Operative load setup
At level load an operative record with flag 0x40 SHALL switch its node's effects off and SHALL still run its kind's setup. A light job whose value is odd SHALL start an electric effect. A fire record of class 8 SHALL light a fire whose size is the low half and whose lift (cm) is the high half of its first target word. A load fire's size SHALL be hundredths of the stored size, at least 0.25 and at most 10.

#### Scenario: A flagged fire
- **WHEN** a fire record with flag 0x40 and a first target word holding 250 and 40 is set up at daylight 0
- **THEN** the effects are switched off and a fire of size 250 with a lift of 40 cm is lit

### Requirement: Daylit fire
A fire record set up when the daylight exceeds 0.12 SHALL light no fire; the first light node under it SHALL be put out, or that node's parent when the parent is lit, or the node itself when there is no light node.

#### Scenario: The light or its lit parent
- **WHEN** a fire record is set up at daylight 0.12 and at 0.13, under a lit node that holds a light node, and again when that parent is not lit
- **THEN** at 0.12 a fire is lit, at 0.13 the lit parent is put out, and without a lit parent the light node is

### Requirement: Signalled fire size
A signalled fire job SHALL take hundredths of its stored value as its size, or 2 when the value is 0, without a clamp.

#### Scenario: A zero size and a large one
- **WHEN** fire jobs of stored size 0, 20 and 2000 are signalled
- **THEN** their sizes are 2, 0.2 and 20

### Requirement: Node word latch
Latching the doors under a node SHALL write the value into bit 0 of the node's word even when there is no door. Closing and locking them SHALL set that bit for every leaf that is already shut.

#### Scenario: A latch with no door, and a shut leaf
- **WHEN** the doors under a node with no door are latched, and two shut sliders under a node whose word is 16 are closed and locked
- **THEN** the first word is 1 and the second is 17

### Requirement: Placed mechanism records
A placed object with a door or an operative section SHALL keep both records under its placed id, tag bits included, and a signal to it SHALL be dispatched by those records. Its body SHALL stay static level geometry while the solver cannot hold a body hung from the world.

#### Scenario: An inverted door and a fire job
- **WHEN** a placed inverted, motorised door is signalled 1, 0 and −1, and a placed fire job of size 200 is signalled 1
- **THEN** the door closes and locks, opens, and toggles, and the fire job starts a fire of size 200

### Requirement: Body grid counts
The system SHALL make a body's voxel grid over its mesh's stored box: the longest half extent (the first of equal ones) gets round(half · resolution) cells, ties to even, made even, at least the minimum depth and at most 96; each other axis gets n · half / longest half rounded up, made even when the builder asks. A longest half extent under 1 cm SHALL make no grid.

#### Scenario: Short axes round up
- **WHEN** a mesh's box has half extents 35 × 10 × 10 cm at 0.3 cells per cm
- **THEN** its grid is 10 × 4 × 4 cells (10.5 rounds to 10; 2.86 rounds up to 3 and is made even)

### Requirement: Builder resolution
The limb and root builders SHALL fit at 0.3 cells per cm when the mesh's smallest half extent is at least 10 cm and at 0.6 below it, with at least 6 cells along the longest axis, or at 1 cell per cm with at least 8 cells under their quality flag.

#### Scenario: A thin mesh
- **WHEN** a mesh's smallest half extent is 9.99 cm
- **THEN** it is fitted at 0.6 cells per cm

### Requirement: Body surface fill
The system SHALL mark every inner cell whose centre is within half a longest-axis cell of the mesh, then every empty cell with at least five marked neighbours whose offsets sum to at most 3 on each axis and whose centre is again within that radius. With the voxeliser's wide-query flag the two radii SHALL be √2/2 and 0.75·√2 of it, taken in double precision.

#### Scenario: The outer layer
- **WHEN** a 20 × 18.8 × 18.2 cm box is fitted over a 21 cm cube's 6 × 6 × 6 grid
- **THEN** exactly the cells of the first and last layer of each axis are marked

### Requirement: Body solid fill
The system SHALL make an empty cell solid when, for all three axes, a ray from its centre toward −axis reaching the mesh box widened by 0.01 cm and a ray toward +axis reaching 0.2 cm past it both hit the mesh.

#### Scenario: A box fills its grid
- **WHEN** the box above is fitted
- **THEN** its inner 4 × 4 × 4 cells are solid, and its 216 cells weigh 9261 cm³ with the centre at the box's

### Requirement: Body sphere query
A sphere SHALL touch a mesh face when the face's plane is no farther than the radius from its centre and the squared distance to the face's nearest point (the original's region test, with the processor's approximate reciprocals) is strictly under the squared radius less 1.192e-7. Speeding the query up SHALL NOT change any answer.

#### Scenario: Strictly inside the radius
- **WHEN** a point lies 1 cm beyond a face's corner in its plane
- **THEN** a sphere of radius 1 cm there does not touch the face and one of the next larger single-precision radius does

### Requirement: Body ray query
A ray SHALL hit a mesh face only when the face's normal points along it by at least 1e-8, the hit is between 1e-8 and the ray's length, and the hit lies in the face by the barycentric test on the two axes other than the normal's largest. Speeding the query up SHALL NOT change any answer.

#### Scenario: One-sided rays
- **WHEN** a ray crosses a face against its normal
- **THEN** it does not hit the face, while the same ray along the normal does

### Requirement: Body candidate spheres
The system SHALL take as candidates the cells 3 or more from the grid's ends whose eight corner cells are filled, with the radius in cells to the nearest cell short of eight filled corners, searched within the cell's distance to the grid's ends, kept when at least 1. Each candidate SHALL count the cells within r − ¼ of its corner point.

#### Scenario: A central sphere
- **WHEN** the 6 × 6 × 6 box above is fitted
- **THEN** its one candidate is the middle cell, 3 cells from the nearest cell short of eight corners

### Requirement: Body sphere pruning
The system SHALL sort the candidates by radius with the original's quicksort and, smallest first, drop any whose every counted cell another candidate also counts, taking its counts back, except the strictly largest. The survivors SHALL count their cells within r + ½ and become spheres at their corner point of radius r/(1 + 0.01·r) longest-axis cells, in double precision, in sorted order.

#### Scenario: Pruning order
- **WHEN** a solid 12 × 6 × 6 cell rod is fitted
- **THEN** its seven candidates of radius 3 are sorted 7, 8, 9, 6, 3, 4, 5 along it, cells 7 and 4 are dropped, and 8, 9, 6, 3 and 5 become spheres in that order

### Requirement: Body corner spheres
After the candidate spheres, every cell up to the last inner one with more than the builder's corner count of filled corners, at least two of them not counted by a sphere, SHALL drop the uncounted corners that a neighbour with more than one more filled corner shares, and give a sphere when more than the builder's free-corner count remain, or, when none remain, only for a cell of 4 to 6 corners whose mask is its own mirror or a face neighbour's.

#### Scenario: Shared corners
- **WHEN** a cell's −x neighbour has its four +x corners filled
- **THEN** those are this cell's four −x corners

### Requirement: Corner sphere placement
A corner sphere SHALL be one longest-axis cell across, at the cell's corner point moved toward its empty side by 0.02 of half a longest-axis cell per unit: −4 per empty corner direction and (8 − corners) times the offset of every neighbour short of eight. A mesh with no candidate SHALL get the corner spheres without the counting.

#### Scenario: A thin slab
- **WHEN** a solid 6 × 6 × 2 cell slab is fitted
- **THEN** each of its 25 cells with eight corners gives a sphere 2 cm across, the middle one at its corner point and the one beside the low x end moved 24 units toward +x

### Requirement: Body weighing
The system SHALL weigh a fitted body with mobility 10 000 over its volume and point masses 4 over the barycentric weights, by the original's triple products, of its centre moved by the builder's origin on its rest tetrahedron.

#### Scenario: Weighted point masses
- **WHEN** the moved centre has weights ½, ¼, ⅛, ⅛ on the rest tetrahedron
- **THEN** the point masses are 8, 16, 32, 32

#### Scenario: Shipped bodies
- **WHEN** the installed game's `Objlib` body nodes are fitted from their meshes (with `EXANIMA_DIR`)
- **THEN** most of them get their stored sphere lists bit for bit, with the fit's version and the voxeliser's wide-query flag they were made with

### Requirement: The hand's grab
A limp body SHALL let a grab pull a point of one part toward a world anchor, right after that part's own links, in each pass up to the grab's last pass (counted from 0 each tick) or in every pass. The pull SHALL move each particle by its weight, taken by the part alone.

#### Scenario: A crate dragged across a floor
- **WHEN** a resting 40 cm crate of mobility 0.5 is grabbed by its middle at strength 1 for the first two passes and its anchor is put 1 m away across the floor for 300 ticks
- **THEN** it never moves more than 10 cm a tick, the grabbed point stops within 5 cm of the anchor with the crate still on the floor, and let go it settles and sleeps

### Requirement: The grab's pull
The gap SHALL be taken across the ground only; under 1.19209e-7 cm² it SHALL do nothing. Otherwise the pull SHALL be twice the strength times the gap, the gap counted at most 3 times the root of the part's mobility in cm and never more than 5 cm. The share of the point's motion since the tick began that heads toward the anchor SHALL be taken off when it is over 1.19209e-7 cm.

#### Scenario: The pull and its cap
- **WHEN** a part of mobility 1 is pulled at strength 1 from 2 cm away (9 cm below), then from 10 cm away, a part of mobility 4 at strength 0.5 from 10 cm away, and a gap of 3e-4 cm
- **THEN** the pulls are 4 cm, 6 cm and 5 cm across the ground, and the last pulls nothing

#### Scenario: Approaching motion
- **WHEN** the point already moved 1.5 cm toward an anchor 10 cm away on a part of mobility 1
- **THEN** the pull is 4.5 cm; moving 1.5 cm away it stays 6 cm

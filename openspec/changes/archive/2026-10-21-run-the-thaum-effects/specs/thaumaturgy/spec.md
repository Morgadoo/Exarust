## ADDED Requirements

### Requirement: Cast state gate
The Force and Energy gate SHALL wait (−3) for an attacking body in a swing state of 0x34 and fail (−2) any other attacking body; fail a moving body that is not stopping, wait for one at a pace of 2 or more and pass one below with 1; ask a casting body for its release and wait; fail a collapsing body; pass a standing body with 0 unless it is switching weapons (wait); fail a body on the ground, and wait for one getting up past 0.9 of its clip.

#### Scenario: The gate's branches
- **WHEN** the gate reads an attacking body with swing bit 4, one with swing bit 2, a stopping body at pace 1.9 and at 2, a casting body and one getting up at 0.9 and 0.91
- **THEN** it answers −3, −2, 1, −3, −3 with the release set, −2 and −3

### Requirement: Displacement gates
The instant Displacement gate SHALL fail a vanished body and, at a clip rate of 1 or more, a collapsing one; the held gate SHALL fail, at that rate, a collapsing or dead one. Both SHALL test the swing as the state gate does, ask a casting body for its release, and otherwise pass on the word the caller last held, the id of the power being cast.

#### Scenario: The held word
- **WHEN** a standing body with power 0x201 charging meets the instant gate, a vanished body meets it, and a lying body meets both gates
- **THEN** the first passes with 0x201, the vanished body fails, and the lying body passes the instant gate with its word and fails the held gate

### Requirement: Persistent slots
Slots SHALL be the highest pick of 2 to 4 among the record's first `thaumaturgy word / 5000` tiers, one less without the focus item, less one for each running persistent power; a cast of a power that already runs SHALL mark it ending and return 2.

#### Scenario: Two tiers and a running bubble
- **WHEN** a record of word 10000 with picks 3, 1, 4 is read, and a body with two slots running a bubble and kinesis casts the shell and then the bubble
- **THEN** it has 2 slots, the shell has 1 free slot, and the bubble's cast returns 2 with the bubble marked ending

### Requirement: Cast motion start
A running cast's hook that answers 0 or more SHALL keep a new cast from starting; otherwise the hook SHALL clear, the power become the one charging, its release clear and, for a motion above 0, that motion play with the condition casting.

#### Scenario: A held cast
- **WHEN** a blast starts against a hook answering 0, then against one answering −1
- **THEN** the first does not start and the second charges 0x201 with motion 0x22 and condition 0x20

### Requirement: Domain prices
A front end SHALL halve an instant power cast again with the repeat technique, price a sustained power at the tick in milliseconds times its cost times 0.001 (Force and Displacement) or over 1000 (Energy), halved again with the sustain technique, and keep a persistent power's cost.

#### Scenario: A torrent tick
- **WHEN** a torrent costing 0.1 a second is priced for a 12 ms tick
- **THEN** it costs 12 × 0.1 × 0.001

### Requirement: Focus factors
Blast, bolt and shock SHALL need 1.2 times the cost and the long door 0.8 times, in double precision; barrier twice and vanish 80 times in single precision. The player's Force and Displacement front ends SHALL refuse while status bit 0x2_0000 is set or a possession runs.

#### Scenario: Short in double precision
- **WHEN** a body with focus 0.12 casts blast, one with 0.1201 casts it, and one with 0.16 casts the long door
- **THEN** the first is refused, the second starts, and the long door is refused

### Requirement: Force casts
Kinesis SHALL release while the controls drag something; a throw SHALL take what a charging kinesis holds, else be refused while the controls drag, and a pick that finds nothing SHALL still return 1; strikes SHALL need a weapon in hand and start motion 0x25; a running barricade SHALL end only when the cursor's ray meets its wall. A throw SHALL take a moving body off the level that nobody holds: a character only when dead, a prop of mobility 0.01 or more or one marked for any weight.

#### Scenario: Strikes and the barricade
- **WHEN** fists cast strikes, a one-handed weapon casts them, and a running barricade is cast again away from and then under the cursor
- **THEN** the fists are refused, the weapon starts motion 0x25, the first barricade cast raises a new wall and the second ends the running one

### Requirement: Barricade placement
A barricade SHALL stand at the cursor's ground point (15 cm back along the hit's normal unless the surface keeps it), 10 cm up; a probe down of 210 cm that meets the floor SHALL move it there, with the wall's middle 100 cm above. It SHALL face away from the caster on the level and reach 300 cm each way; its placing sound's pitch SHALL be 0.8 + 0.4 times a roll.

#### Scenario: On the floor
- **WHEN** the cursor hits (100, 0, 0) with an upward normal, the floor lies 20 cm below and the caster stands at (0, 50, 0) with a roll of 0.5
- **THEN** the middle is (100, 75, 0), the facing +x and the pitch 1.0

### Requirement: AI advice
The AI SHALL be told 1 for a blast with the focus, facing within 1 and the target on its feet within 6 m; 0x10 for a barrier with half the focus above the cost, facing 1 or more, within 3 m; 3 for a shock within 5 m; and 0x10 for shell, strikes, constant displacement and shock weapons when the cost is within half the focus, facing within 0.5, the power not running and a slot free, else 0x400_0000.

#### Scenario: Range
- **WHEN** the AI weighs a blast at 360000 cm² and at 360001 cm²
- **THEN** it is told 1 and 0

### Requirement: AI cost words
The AI SHALL be told `round(160·cost) << 8 | 4` for the long door against a target on its feet and not vanished, and `round(200·cost) << 8 | 0x800_0000`, with 0x20 at three quarters health or less, for the summoning, and 0x400_0000 while a summoned creature lives.

#### Scenario: The words
- **WHEN** the AI weighs a long door costing 0.2 with focus 0.26, and a summoning costing 0.35 at three quarters health
- **THEN** it is told 0x2004 and 0x800_4620

### Requirement: Energy casts
Shock SHALL start motion 0x27 and be made only when the motion starts, returning 1 either way; its charge SHALL last the cast clip's milliseconds over 1200 and cost the cost times that times 1.2. Shock weapons SHALL need a weapon and a slot, arm at most four weapons, end a put-away weapon's arcs and arm a taken one while fewer than four are armed.

#### Scenario: A full clip and a fifth weapon
- **WHEN** a 1200 ms clip charges a 0.1 shock and four armed weapons see a fifth taken and one put away
- **THEN** the charge lasts 1 and costs 0.12, the fifth is kept off, and the put-away weapon's arcs end

### Requirement: Phasing
A constantly displaced body SHALL let a blow through when a percentage roll is at most `100 >> (s + 1)` for a streak s of 0 or more, or `100 − (100 >> (1 − s))` below 0; a pass SHALL raise a non-negative streak (a negative one restarts at 0) and a miss lower a non-positive one (a positive one restarts at 0). A passed weapon SHALL get its collisions back once more than 500 ms have passed.

#### Scenario: The odds
- **WHEN** the streak is 0, 1, 2, −1 and −2, and weapons passed at 1000 and 1200 ms are restored at 1500, 1501 and 1701
- **THEN** the thresholds are 50, 25, 12, 75 and 88, and the weapons come back at 1501 and 1701

### Requirement: Arc tendrils
Arcs SHALL keep `round(segments·life)` points a tendril, lose 0.1 of life a tick once their timer runs out, open a branch where the chord over a point is 0.6 of a live segment or more, grow one point a tick toward their length, keep 0.9 of their motion with a 0.02-segment jitter, and end at the level where a segment meets it. A respawning emitter SHALL start a free record half the ticks with a life of `life` to `2·life` ticks.

#### Scenario: A single arc
- **WHEN** a non-respawning arc of 4 segments with a 2-tick life ticks 3 times, and a tendril of 4 points along x meets the level at x = 6
- **THEN** its life is 0.9 after the third tick and the cut tendril keeps 3 points, the last at x = 6

### Requirement: Arc strikes
A segment that meets a limb of a character other than the caster SHALL shock that character with the emitter's strength along the line from the arcs' mean point to its chest, once with the shock-once flag, and send a branch toward the limb.

#### Scenario: Once only
- **WHEN** a tendril's first segment meets a character's limb twice with the shock-once flag
- **THEN** the first meeting shocks the character and branches toward the limb, the second does not shock

### Requirement: Arc frame
Each frame the light SHALL flicker within 0.75 to 1.2, the hum's gain be fill² times its flicker (0.5 to 1.5) times the volume and its pitch 0.5 plus half the fill times its flicker (0.8 to 1.1); the ribbon SHALL pair every point and Hermite midpoint, its first and last pair transparent, and drop its last triangle.

#### Scenario: A three-point ribbon
- **WHEN** a three-point tendril along x is drawn facing +z
- **THEN** it has 12 vertices, 27 indices, and its first quad is 0, 1, 3, 0, 3, 2

### Requirement: Barricade rise and fall
A barricade's reaches SHALL be trimmed 40 cm past the level; it SHALL rise over 1.6 s with its span `min(e·300, reach)` each way, `e = u(2 − u)·0.9 + 0.1`, `u = t(2 − t)`, its half width that less 15; told to end, it SHALL fall over 750 ms from where it stands with its half height `(1 − t⁴/2)·100 − 10`, and leave the grid and the scene at the end. Its power SHALL end once its caster is more than 20 m away.

#### Scenario: Half way up
- **WHEN** a barricade has risen for 800 ms
- **THEN** its half width is 268.125 and its half height 90

### Requirement: Barricade shove
A sphere inside the wall's rounded slab SHALL be pushed along its normal by `((r + 10) − |z|)·0.04 + 0.1`, at most 0.5, times `m/(m + 1)`, a tenth from behind, and strike a ripple of three times the push. A character's limb in front within 30 times its scale of the plane SHALL be knocked back by twice 40 along the normal (at most its step), its instability set to 100 and its energy above 0.25 lowered by `(30 − d)·0.0002 + 0.002`.

#### Scenario: A limb in front
- **WHEN** a sphere of 10 cm 5 cm in front of a standing wall, of mobility 1, belongs to a character whose chest is 20 cm in front
- **THEN** it is pushed 0.25 along the normal, knocked back 80, its energy lowered by 0.004 and a ripple of 1.5 starts

### Requirement: Wisp strands
Each strand SHALL keep a ring of eight nodes, start a new head once its head is more than 40 cm from its anchor, light it with its budget of 166 (ramped by `q(2 − q)` over 80 ticks for a body set), spend 50 of its budget a node for a blow's set, darken each node by 1 a tick and by 2 to 6 for the oldest four, grow it by 0.1 and keep 0.9998 of its motion.

#### Scenario: The darkening
- **WHEN** a stopped strand's eight nodes at light 100 tick once
- **THEN** from the head back they read 99, 99, 99, 99, 98, 97, 96, 95

### Requirement: Live effects
The game SHALL run live arcs, barricades and wisps after the bodies; a shocked fighter SHALL lose the strength in yellow health at most once every 150 ms; a barricade's push SHALL move a part's particles by their weights of the struck sphere's centre and restore its edges past the push strain. `--debug-arcs`, `--debug-barricade` and `--debug-wisps` SHALL put the effects on the player.

#### Scenario: Two shocks within the gap
- **WHEN** a fighter is shocked by 0.05 at 0 ms, at 144 ms and at 156 ms
- **THEN** its yellow health falls from 0.25 to 0.20, stays, then falls to 0.15

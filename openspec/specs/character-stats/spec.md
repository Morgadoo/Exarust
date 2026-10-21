# character-stats Specification

## Purpose
Gives every character the state and stats the original keeps on it: the
race's flag words and the stats its init, combat, AI and sounds read, the
character's own record read whole with the perks its techniques give, the
stats, health and rule bits derived from both at spawn, the encumbrance
what it wears and holds gives it and what that slows, the condition word
the body tick writes and its readers test, and later regeneration.

## Requirements

### Requirement: Race flag words
The system SHALL decode, for every record of the installed `races.rdb`, the second flag word at `0x1C` and the kind word at `0x58` beside the flags at `0x18`, and tell from them what the original tests: red regeneration (bit `0x80` of the first word or `0x4` of the second), rising again in the story (`0x8` of the first), hovering feet (`0x200` of the first), a quadruped (`0x8` of the second) and claws (`0x100` of the kind word).

#### Scenario: Ogre
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ogre's words are `0x27`, `0x7` and `0x20001`: it regenerates red health and fights with plain fists

#### Scenario: Clawed races
- **WHEN** the installed `races.rdb` is parsed
- **THEN** exactly Sir, Fael, Ghoul, Ghoul H, Gobbler and the harvester fight with claws

#### Scenario: Regenerating races
- **WHEN** the installed `races.rdb` is parsed
- **THEN** exactly Ogre, Sir, the harvester, the demon and the golem suit regenerate red health

#### Scenario: Wraith and harvester
- **WHEN** the installed `races.rdb` is parsed
- **THEN** only the wraith hovers and only the harvester is a quadruped

### Requirement: Race stats
The system SHALL decode, for every record of the installed `races.rdb`, the stats the original's character init, combat, AI and sounds read: the unarmed reach, the held weapons' swing-turn and recovery factors, the AI attack bonus, the perception, the held-item offset, the footstep sound set, the voice volume and the attack voices. A missing race SHALL get the original's fallback: 1 m reach, factors 1, no attack bonus, no offset, no footsteps, a silent voice.

#### Scenario: Human
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Human has unarmed reach 0.5 m, swing turn 1, recovery 1, attack bonus 0, perception 1, no held offset, footsteps `StpSft`, voice volume 1 and 4 attack voices

#### Scenario: Big races
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ogre has reach 1.0 m, swing turn 0.5, footsteps `StpHvy` and voice volume 3; Sir reach 1.5 m and swing turn 0; Fael reach 1.3 m, the only attack bonus (0.2) and footsteps `StpMed`

#### Scenario: Undead
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Skeleton has perception 0, footsteps `SkelStp` and no attack voices; Ancient perception 0.5; Wraith perception 0

#### Scenario: Golems and the demon
- **WHEN** the installed `races.rdb` is parsed
- **THEN** every golem has recovery 0.5, footsteps `Golem_Stp`, voice volume 0.5 and no attack voices, and the demon alone holds items 9 cm off their grip

#### Scenario: Unknown race
- **WHEN** a character's race byte names no race in the database
- **THEN** its stats are the fallback: 1 m of unarmed reach, no footsteps and voice volume 0

### Requirement: Character records
The system SHALL read every character record whole, versions 5 to 7 as the original's readers do: physique, height, age, voice, held and worn items, then the feral level, the saved yellow, red and focus pools, combat skill, undead voice, traits, experience, six skills with the technique chosen at each level, inventory, roles, relations, saved body state and powers. A record running past its size, or an entry that is not a record, SHALL be an error.

#### Scenario: Level databases
- **WHEN** the character sections of every level in `Resource.rpk` are read
- **THEN** the 478 version 7, 20 version 6 and 4 version 5 records of the 13 sections parse to their ends, and the 28 entries that are not records (26 deleted, 2 stubs) are errors

#### Scenario: Placed characters
- **WHEN** every level is read
- **THEN** each of the 477 placed characters has a version 7 record, each of the 451 living ones stores full pools, and each of the 97 living humans is feral (0.4 to 1.0) while no other race is

#### Scenario: Arena actors
- **WHEN** the records of `actors.rpk` are read
- **THEN** all 21 parse, 14 of them version 6

#### Scenario: Legacy roles
- **WHEN** a version 5 or 6 record's first role is Disposition, XA_Proctor or Skeleton - Wait
- **THEN** it holds Corpse, XA Undead Bandit or Skeleton respectively, with Activity Wait added after it, as the original's legacy reader does

#### Scenario: Saved body state
- **WHEN** a record saves a heal pool of 0.25 and a potion expiring at 1800
- **THEN** the heal pool reads 0.25 and the potion 1800, each from its own entry

### Requirement: Perks from techniques
The system SHALL learn one technique per 5000 points of a skill, at most six, and give from the learnt ones the perks the original reads: Remise, Feint, Fend, Riposte (Close Combat 1–4), Security (Armour 5), Counter, Synchrony, Ward, Persist, Endure, Mobility (Shields 1–6), Iteration (Insight 6), Sustain, Abstraction, Clarity, Volition (Concentration 1, 5, 6, 8); also the highest Manoeuvring in Armour tier learnt (0–3) and Deflection (Armour 4).

#### Scenario: A level not yet reached
- **WHEN** a Close Combat skill has 15000 points and the techniques Remise, Riposte, Feint and Fend
- **THEN** the character has Remise, Riposte and Feint, not Fend

#### Scenario: Armour tiers
- **WHEN** Armour's learnt techniques are Manoeuvring in Armour I and II
- **THEN** the tier is 2; with all three and Deflection learnt the tier is 3 and Deflection is learnt

#### Scenario: Techniques without perks
- **WHEN** a character learns Dual Wield, Draw, Impel or Brace
- **THEN** it gains no perk from them

#### Scenario: Arena champion
- **WHEN** `actors.rpk` is read
- **THEN** opponent16 has Remise, Feint, Fend, Riposte and Security, Manoeuvring tier 3 and Deflection

#### Scenario: Story levels
- **WHEN** every level record is read
- **THEN** 358 records have Remise, 147 Security, 103 Counter and 85 Mobility, and 37 have learnt Dual Wield

### Requirement: Stats derived at spawn
The system SHALL derive, when a character spawns, from its record and race as the original's builder and init do: height kept within −1…1 and feral level f within 0…1; body scale min(race scale, 1.5)·(1 − 0.05·height); size race size·(1 − 0.05·height); standing pelvis 109.5 − 11.25·height cm; leg reach 107.2 − 11.1·height cm; most energy 1 − 0.1·f; support pins' factor race − 0.25·f²; step reach 80 cm·√scale·(1 − 0.3·f)·(1 − 0.2·encumbrance).

#### Scenario: Short and tall humans
- **WHEN** a human of height 1, and one of height −1, spawns
- **THEN** the first has scale and size 0.95, its pelvis 98.25 cm up and leg reach 96.1 cm; the second scale 1.05, pelvis 120.75 cm and leg reach 118.3 cm

#### Scenario: Capped scale
- **WHEN** a harvester (scale and size 1.8) of height −1 spawns
- **THEN** its scale is 1.575 and its size 1.89

#### Scenario: Zombie
- **WHEN** a human of feral level 1.0 spawns
- **THEN** its most energy is 0.9, its support pins' factor 1.0 and its step reach 56 cm (80 cm for a living human)

#### Scenario: Story levels
- **WHEN** every placed character of the installed story levels spawns
- **THEN** the 97 living humans have most energy 0.9 (×47), 0.9158 (×21), 0.95 (×27), 0.9576 and 0.9602, their scales lie from 0.95 to 1.05, and every other race keeps most energy 1

### Requirement: Undead control and voice
The system SHALL derive for a feral level f: the head's pose gain × (1 − f), its muscle factor × (1 − 0.8·f), the arms' stiffness in the walk and run cycles 0.2 − 0.15·f (at least 0.05), the sprint gait, perception 1 − f/2 and an attack-voice step of 1; and for every character a voice pitch kept within 0.9…1.07 (the hulking ghoul 0.85), a voice volume of scale × the race's factor and the race's clip-rate base.

#### Scenario: Zombie and half-feral heads
- **WHEN** humans of feral level 1.0 and 0.5 spawn
- **THEN** the first's head has no pose gain, a fifth of its muscle factor and walking arms 0.05; the second's half its gain, 0.6 of its factor and walking arms 0.125

#### Scenario: Voices
- **WHEN** a human's record stores pitch 0.8, another 1.2, and a Fael (scale 1.35, voice 1.5) spawns
- **THEN** the pitches are 0.9 and 1.07 and the Fael's voice volume 2.025

### Requirement: Health at spawn
The system SHALL give a spawning character maximum health 0.25 × its race's health factor, but by race byte: the ogre 1.5, Sir 1.0, golems 0.5 and the golem boss 1.0, the ancient, the ghoul and the manipulator 0.375, the Fael 0.75, the hulking ghoul and the gobbler 0.5. Its yellow and red health SHALL be the saved pools' shares of it, both full for a living character whose record saved no red health.

#### Scenario: Manipulator
- **WHEN** a manipulator (race health factor 1) spawns
- **THEN** its maximum health is 0.375

#### Scenario: Saved pools
- **WHEN** a human's record saves yellow 0x8000 and red 0xFFFF
- **THEN** it spawns with yellow 0.125 and red 0.25

#### Scenario: Arena opponents
- **WHEN** the arena's opponents (records saving no health) spawn alive
- **THEN** each has full yellow and red health, a human's 0.25; a corpse with an empty record keeps none

#### Scenario: Story levels at full health
- **WHEN** every placed character of the installed story levels spawns
- **THEN** each of the 451 living ones has full health

### Requirement: Body rule bits
The system SHALL set a spawning character's rule bits as the original's init does: remedies for living humans not feral and for creatures; may rise for those humans in the story; no lasting wounds for feral humans, skeletons and ancients; fierce over half feral and for creatures; creature hands, red regeneration and steadfastness by race byte and the race's flag words; no red regeneration for golems not wearing their kind's core.

#### Scenario: Humans
- **WHEN** a living human spawns in the story, in the arena, and a human of feral level 0.84 in the story
- **THEN** their rule bits are 0x12, 0x2 and 0xC

#### Scenario: Creatures and golems
- **WHEN** the ogre, the Fael and a golem guard without its core spawn, and every placed golem of the story levels (each wearing its core)
- **THEN** their rule bits are 0x67, 0x7 and 0x140, and 0x40 for the placed golems

### Requirement: Encumbrance
The system SHALL sum a character's encumbrance at spawn as the original's armour pass does: 0.0025 per unit of each worn item's encumbrance (an unpowered golem suit adds 2), less 0.025 for each articulated piece touching another, less 0.3 per Manoeuvring in Armour tier, none below 0.001; then the held shield's (2 − weightless) × bulk / 4, less 0.3 with Mobility, when positive; at most 1.

#### Scenario: The duellist's outfit
- **WHEN** the duellist spawns in its leather jacket, trousers, boots and full helm
- **THEN** its encumbrance is 0.175 and its step reach 77.2 cm

#### Scenario: A plate set
- **WHEN** the player spawns with `--outfit fptdhrnsa,fptdhelma,fptdlegsa`, a tourney harness, helm and leggings of one articulated set
- **THEN** its encumbrance is 0.625: 0.7 less 0.025 for each of the three touching pieces

#### Scenario: Techniques and a shield
- **WHEN** a record wears a mail shirt of encumbrance 40 and holds a sword and a round shield of bulk 0.6
- **THEN** its encumbrance is 0.4; 0.3 with Manoeuvring in Armour I; 0 with Mobility as well

#### Scenario: Golem suit
- **WHEN** a character wears an unpowered golem suit
- **THEN** its encumbrance is 1, whatever its Manoeuvring tier

#### Scenario: Story levels
- **WHEN** every living character of the installed story levels spawns
- **THEN** 386 of the 451 weigh nothing, 27 hold a shield, and Elestan (0.77, with a round shield) and Demian (0.67) are the most encumbered

#### Scenario: Arena actors
- **WHEN** the arena's actors spawn in their own items
- **THEN** opponent07 weighs 0.475 (0.175 of armour and a round shield's 0.3), opponent13 0.07 (Manoeuvring II takes its armour off, its buckler stays) and the demon nothing

### Requirement: Encumbered movement
A character's step reach SHALL shrink by 20 % per unit of its encumbrance, and the player's run blend SHALL rise at most to 1 − 0.3 × its encumbrance.

#### Scenario: Step reach
- **WHEN** a human of scale 1 spawns at encumbrance 0.5
- **THEN** its step reach is 72 cm

#### Scenario: Running in plate
- **WHEN** the player wearing the 0.625 plate set runs to a far cursor
- **THEN** its run blend stops at 0.8125 and it runs at about 3.2 m/s instead of 3.6 m/s

### Requirement: Condition word
The system SHALL keep on every fighter the original's condition word: 0 normal, 1 walking, 2 attacking, 8 combat idle, 0x20 casting, 0xC0 shocked, 0x100 weak, 0x200 down, 0x300 getting up, 0x600 dormant, 0x800 collapsing, 0xA00 dead, 0x8000 kicking, 0x10000 and 0x10004 switching weapons; and test it by the original's masks 0xAC0 (helpless), 0xA00, 0x200, 0x800 (dying) and 0x10203.

#### Scenario: Masks
- **WHEN** the masks are tested against every value
- **THEN** 0xAC0 holds shocked, down, getting up, dormant, collapsing and dead; 0xA00 the same but shocked; 0x200 down, getting up, dormant and dead; 0x800 collapsing and dead

#### Scenario: Placed corpses
- **WHEN** `play exanima01` places its dead characters
- **THEN** each corpse's condition is dead

### Requirement: Posture decision
In each living character's posture stage, before its clip advances, the system SHALL write its condition: a dazed one normal unless helpless or attacking; then collapsing, dead and shocked kept; getting up kept; under a balance of 0.6 (double precision) down below 0.3, else kept; an attack clip attacking; a tone under 0.85 weak; else by its clip: the kick, the combat idle (or a parry), a power, the walk or the run, a weapon switch (switched after 60 %), else normal.

#### Scenario: Thresholds
- **WHEN** a body in the combat idle has a balance of 0.6, 0.599 and 0.299 in turn
- **THEN** its condition is 8, still 8, then 0x200; at 0.3 a walker stays 1

#### Scenario: Attacks and tone
- **WHEN** a standing body plays an attack at tone 0.1, then the combat idle at tone 0.84 and 0.85
- **THEN** its condition is 2, then 0x100, then 8

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` deals the player a body blow
- **THEN** the trace's condition reads normal while it falls, 0x200 once its balance is under 0.3, 0x300 from the get-up's start and normal again when the get-up clip ends

#### Scenario: Stagger
- **WHEN** a body off balance staggers
- **THEN** its condition is 0x300 from the stagger's start until its get-up clip ends

### Requirement: Condition events and readers
A get-up's start SHALL write getting up and the end of its clip normal; a killing blow SHALL write collapsing (dead without muscles). Hit detection SHALL skip the dead; the daze SHALL leave a helpless or attacking fighter's clip alone; the player SHALL not be steered nor engage an attack while helpless; the duellist SHALL idle for 0xA00 and leave a dying target; the world cursor SHALL read dead, collapsing and bit 0x200 from the word.

#### Scenario: Killing blow
- **WHEN** a cut takes a fighter's yellow health under 0.01
- **THEN** its condition is collapsing, a further cut leaves its collapse as it is, and it is dead 250 ticks later

#### Scenario: Dazed attacker
- **WHEN** a fighter is dazed during its attack
- **THEN** its condition stays attacking and its attack clip plays on

#### Scenario: Cursor
- **WHEN** the player's condition is getting up, then collapsing, then dead, in combat mode
- **THEN** the cursor is the ring while it gets up and collapses and the arrowhead once dead

### Requirement: Collapse before death
A killing blow on a living body SHALL start its collapse: condition collapsing, Fall Backward blending round((1.2 − E) × 300) frames, smooth, E the energy the blow leaves; not carried. Each tick, in place of the posture decision: its heading drifts by up to 0.25 per axis; under size 1.2 and balance 0.8 Rebalance starts (20 frames); tone −0.003, pins' gain −0.0003. At tone 0.25 it SHALL die: dead, limp, keeping its motion.

#### Scenario: Blend
- **WHEN** a killing blow leaves a fighter's energy at 1, 0.62 or 0.05
- **THEN** Fall Backward starts with 60, 174 or 345 frames of blend

#### Scenario: Three seconds
- **WHEN** a fighter at full tone starts its collapse
- **THEN** it is collapsing for 249 ticks and dead on the 250th, at a tone of 0.24999

#### Scenario: Big bodies
- **WHEN** a collapsing body of size 1.2 loses its balance
- **THEN** it does not Rebalance, where one of size 1 does once its balance is under 0.8

#### Scenario: Killed in the duel
- **WHEN** the duellist kills the player in `exarust --opponent --trace t.csv play arena`
- **THEN** the trace reads 0x800 from the blow, Fall Backward then Rebalance, and 0xA00 250 ticks later, and the body lies limp on its back

### Requirement: Health regeneration
Each fixed tick a fighter not dead SHALL regenerate yellow health as the original: its rate is 0 while it attacks (condition 2) or plays Cmbt - Idle, else it rises 0.0003 × 0.01/83 a tick up to 0.01/83, and yellow gains √red × rate. Every blow and weapon contact SHALL set the rate to 0. A heal pool SHALL move 1.25e-5 a tick to red; yellow SHALL stay within red and red within the maximum. Death SHALL set yellow to 0.

#### Scenario: The retail trace
- **WHEN** a fighter with yellow 0.085 and red 0.203 of 0.25 leaves Cmbt - Idle for the Idle clip
- **THEN** its yellow is 0.0864 after 5 s, 0.1209 after 25.3 s (2,100 ticks) and 0.2009 after 45.8 s, the retail trace's 0.086, 0.121 and 0.202 within 0.0012

#### Scenario: The ramp
- **WHEN** a fighter's rate starts from 0
- **THEN** it reaches 0.01/83 on the 3,334th tick (40.2 s), and at it a human with full red heals 0.005 a second

#### Scenario: Held
- **WHEN** a wounded fighter attacks, or plays Cmbt - Idle at any tone
- **THEN** its rate is 0 and its yellow does not rise; in the Defence stance it heals

#### Scenario: A blow
- **WHEN** a blow lands on a fighter healing at the full rate
- **THEN** its rate is 0 and the ramp starts again

#### Scenario: Collapse and death
- **WHEN** a killing blow leaves a fighter's yellow at 0
- **THEN** it regains 0.00056 over the 249 ticks of its collapse, and its yellow is 0 from its death on

### Requirement: Red regeneration
Each fixed tick, dead or alive, a fighter whose rule bits hold red regeneration (0x20) without the bit that blocks it (0x100) SHALL regain 1.25e-5 red health, up to its maximum.

#### Scenario: Ogre
- **WHEN** an ogre (rule bits 0x67) with red 0.1 of 1.5 ticks 83 and 1,000 times
- **THEN** its red is 0.10104, then 0.1125

#### Scenario: Golem suit and humans
- **WHEN** a golem suit without its core (0x170), with its core (0x70) and a human (0x12) tick
- **THEN** only the suit with its core regains red

#### Scenario: Shipped characters
- **WHEN** the installed story levels' living characters and the arena's actors spawn
- **THEN** Sir and the three harvesters regenerate red in the story levels, and only the demon in the arena

### Requirement: Damage to health
Damage (yellow Y, red R, vital or not) SHALL take health as the original, subtracting in double precision: the rate to 0; yellow −0.1Y (−0.05Y with a potion); for rule bit 0x8 yellow −0.2R and red kept, else red −0.2R and yellow scaled by red's share left (0 with none). Under 0.05 the low-health check SHALL run, unless vital damage leaves yellow under 0.01 (double): yellow 0, in the story red −0.1 × maximum, killed. Non-vital damage SHALL leave yellow 0 and the fighter alive.

#### Scenario: Order
- **WHEN** a fighter at full health 0.25 takes Y 1 and R 0.5
- **THEN** its red is 0.15 and its yellow 0.09: cut by 0.1, then scaled by 0.6

#### Scenario: No lasting wounds
- **WHEN** a feral human (rule bits 0xC) and a human take Y 0.5 and R 0.3 from full health
- **THEN** the feral one has yellow 0.14 and red 0.25, the human yellow 0.152 and red 0.19

#### Scenario: Potion
- **WHEN** a human whose state flags hold the potion takes the same damage
- **THEN** its yellow is 0.171 and its red 0.19

#### Scenario: The lines
- **WHEN** vital damage of nothing reaches a fighter at yellow 0.01 (single precision), at 0.05 and just under 0.05
- **THEN** the first is killed, the second only hurt, and the third runs the low-health check

#### Scenario: Not vital
- **WHEN** non-vital damage leaves a fighter's yellow under 0.01
- **THEN** its yellow is 0, it lives, and the low-health check runs

#### Scenario: Story
- **WHEN** a killing blow lands on a fighter with red 0.2 of 0.25 in the story, and in the arena
- **THEN** its red is 0.175, and stays 0.2

#### Scenario: Story levels
- **WHEN** every living character of the installed story levels takes Y 0.5 and R 0.3
- **THEN** 248 of the 451 keep their red health (the 97 feral humans, 128 skeletons and 23 ancients) and the other 203 lose 0.06 of it

### Requirement: Falling to death
Each tick a living body in the falls branch SHALL run the fall check: at −5 cm a tick or slower (its chest's slow vertical velocity) it is steady; faster, it screams once (not if dead); then lost if its pelvis is 2,500 cm or more over the ground under it and under each foot. More than 1,000 ms after (game clock) the player hears the pit and its screen fades; it dies after 4,000 ms, anyone else after 3,000: dead, health 0, out of the world.

#### Scenario: Thresholds
- **WHEN** a body sinks at 5 cm a tick, then 5.01, then 4
- **THEN** it is steady, screams once, then is steady and screams again the next time

#### Scenario: Twenty-five metres
- **WHEN** a falling body's pelvis is 25 m over the ground under it and its feet, then 24.99 m, or one foot is over ground 2 m down
- **THEN** it is lost only in the first case

#### Scenario: Timelines
- **WHEN** the player and another character are lost
- **THEN** the player hears the pit on the 84th tick after (1,008 ms) and dies on the 334th; the other dies on the 251st, hearing nothing

#### Scenario: Dropped from 200 m
- **WHEN** `exarust --at 0,200,0 play arena` drops the player
- **THEN** it screams and is lost on the same tick, the pit sounds 84 ticks later and it is gone 334 ticks after the loss

### Requirement: Kill plane
A dead character's body whose root falls below −50 m SHALL lose its health and, unless it is the player's, leave the world with everything it carries; once. A living body, or one already lost to its fall, SHALL not be taken.

#### Scenario: A corpse falls off the world
- **WHEN** a dead character's body, the dead player's and a living body fall below −50 m
- **THEN** the corpse is gone, the player's body stays with no health, and the living body is left alone

### Requirement: Writing a character record
The system SHALL write a character record as the original's writer does: version 7, the fields in file order, names one byte per character, a skill only when it has a chosen technique and at most five techniques, the powers only with an entry, the reserved pair only when its first word is set and the loadouts only when there is one. Every version 7 record of the shipped level databases and `actors.rpk` SHALL write back byte for byte except the voice names that are not in the formatter's form.

#### Scenario: Shipped records
- **WHEN** the 485 version 7 records of the installed levels and actors are written back
- **THEN** 483 are byte for byte the same and the other two differ only in a stored voice name `HM04` written as `vi04`

### Requirement: Trait and voice fields written
The system SHALL write each trait as the nearest-even conversion plus 100 in a byte (NaN and values out of range storing 100) and the voice name as `vi` and the voice number plus one by the original's formatter (two digits at least, `00` for 0, `0` for a negative sum).

#### Scenario: Traits and voice
- **WHEN** a record with traits 2.5, −100.5, 300 and NaN and voice numbers 0, 99 and 2³²−1 is written
- **THEN** the trait bytes are 102, 0, 144 and 100 and the voice names `vi01`, `vi100` and `vi00`

### Requirement: Record defaults
The system SHALL start a new record blank with yellow and red full, focus 0 and voice pitch 1, and the character creator's reset SHALL set a male human, physique 0.5, skin 0x8000, hair style 0, hair colour `40 80 40 00`, voice 0, pitch 1, yellow and red full, and append the underwear, leaving every other field as it was.

#### Scenario: Reset of a record in use
- **WHEN** a record with a name, focus 0x1234 and one worn item is reset
- **THEN** it keeps the name and focus and wears that item and then the underwear

### Requirement: Saved health
The system SHALL save yellow and red as value / maximum × 65535 and focus × 65535 in single precision, converted to the nearest even integer and cut to 16 bits (red 0 unless positive), with the seed 0xFFFF and the heal pool and potion entries; it SHALL load them, and spawn characters with them, as max × (stored × 1/65535) in double precision, every known entry taking the last entry's value.

#### Scenario: Double-precision load
- **WHEN** a share of 257 is loaded onto a maximum of 0.25
- **THEN** the health is the `f32` nearest 0.25 × 257 / 65535, not the single-precision product

#### Scenario: Save
- **WHEN** a body at half yellow, a quarter red and focus 0.75 of maximum 1 is saved
- **THEN** the pools store 32768, 16384 and 49151

### Requirement: Equip conflicts
The system SHALL refuse an equip without change when a worn item that overlaps the new item on any bone has an equip lock over 50, and otherwise take every overlapping worn item off, in order.

#### Scenario: Golem suit
- **WHEN** a character wearing the shipped golem suit equips an apparel item that overlaps it
- **THEN** the equip is refused and the suit stays on

### Requirement: Trading an item away
The system SHALL take the last inventory copy not marked equipped, else the last copy, else the worn copies through the original's skipping loop, and then clear the surplus hand and loadout references in both hand pairs.

#### Scenario: Trade
- **WHEN** an item held as an equipped and an unequipped inventory copy is traded away
- **THEN** the unequipped copy goes

### Requirement: Unlearning a level
The system SHALL remove a chosen level's technique, then the techniques of the same skill that depend on a removed one by the original's prerequisite table, and take 5000 points per technique removed in 16-bit arithmetic.

#### Scenario: Manoeuvring in Armour
- **WHEN** Manoeuvring in Armour I is unlearnt from Armour holding I, Deflection, II, III and Security at 25000 points
- **THEN** Armour holds Deflection and Security at 10000 points

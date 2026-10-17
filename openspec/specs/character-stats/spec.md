# character-stats Specification

## Purpose
Gives every character the state and stats the original keeps on it: the
race's flag words and the stats its init, combat, AI and sounds read, the
character's own record read whole with the perks its techniques give, the
stats, health and rule bits derived from both at spawn, the encumbrance
what it wears and holds gives it and what that slows, and later the
condition and regeneration.

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

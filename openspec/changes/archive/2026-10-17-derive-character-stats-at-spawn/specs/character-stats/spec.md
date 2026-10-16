## ADDED Requirements

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

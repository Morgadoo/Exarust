# character-stats Specification

## Purpose
Gives every character the state and stats the original keeps on it: the
race's flag words and the stats its init, combat, AI and sounds read, the
character's own record read whole with the perks its techniques give, and
later the condition, the stats derived at spawn and regeneration.

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

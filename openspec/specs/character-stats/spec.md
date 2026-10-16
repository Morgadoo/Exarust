# character-stats Specification

## Purpose
Gives every character the state and stats the original keeps on it: the
race's flag words and the stats its init, combat, AI and sounds read, and
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

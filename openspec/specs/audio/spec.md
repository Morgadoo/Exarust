# audio Specification

## Purpose
Plays the original game's sounds for music, ambience and gameplay events.

## Requirements

### Requirement: Level music
The system SHALL loop a music or ambience track chosen from the level name when a level is played.

#### Scenario: Arena
- **WHEN** an arena level is played
- **THEN** an arena music track loops

### Requirement: Hit sounds
The system SHALL play a weapon impact and a pain sound at the contact point of every hit.

#### Scenario: Sword hit
- **WHEN** a sword strike lands on a fighter
- **THEN** a sword impact and a grunt play from the contact point

### Requirement: Footsteps
The system SHALL play footstep sounds at a rate following the speed of moving, grounded characters.

#### Scenario: Running
- **WHEN** a character runs
- **THEN** heavier footsteps play more often than when walking

### Requirement: Sound sets
The system SHALL name a sound set's variants as the original does, the set's prefix followed by a two-digit number from 01, and every sound event SHALL pick its variant through one shared picker: sword impacts `wh_sworda01`–`06`; grunts `vhmNN_grunts01`–`06` for voices 01–09; footsteps `stpmed01`–`06`, and `stphvy01`–`06` running.

#### Scenario: Variant names
- **WHEN** the sword impacts' first and sixth variants are named
- **THEN** they are `wh_sworda01` and `wh_sworda06`

#### Scenario: Within the set
- **WHEN** footsteps pick 500 variants of a six-variant set
- **THEN** every pick is one of its variants 01 to 06

#### Scenario: Installed sets
- **WHEN** the installed `Sound.rpk` is read
- **THEN** it holds every variant of the sword impacts, of the nine voices' grunts and of both footstep sets, and no seventh variant or tenth voice

### Requirement: Fall sounds
A body falling to its death SHALL scream once with a heavy groan at its chest, at gain 0.2–0.35 and pitch 0.95–1.05 times its voice volume and pitch (the pitch from the first of two draws); the player SHALL hear `pitdeath01` at gain 0.1, not placed in the world, when the pit sounds.

#### Scenario: Draws
- **WHEN** the scream's two draws are 0 and 1, then 1 and 0.5
- **THEN** its gain is 0.35 and pitch 0.95, then 0.275 and 1.05

#### Scenario: The pit
- **WHEN** the player is lost and more than a second passes
- **THEN** `pitdeath01` plays once

### Requirement: Story level music
A story level SHALL play the track its name's bytes pick, tested in order: byte 11 `w` Msc_SharpNatural (volume 0.1); bytes 7 and 8 `67` Msc_GardensA (0.3); byte 8 `4` Msc_UWB2_Amb (0.15); `6` Msc_UWB5_Amb (0.25); `5` Msc_UWB4_Amb (0.2); `7` Msc_CountfourA (0.1); `8` Msc_UWB6_Amb (0.1); `2` Msc_UW01AmbB (0.15); byte 7 `c` Msc_CryptAmb (0.25); else Msc_UW01Amb (0.1).

#### Scenario: Crypt and second level
- **WHEN** exanimac and exanima02 are played
- **THEN** they play msc_cryptamb and msc_uw01ambb

### Requirement: Story ambience
A story level whose byte 8 is `4` SHALL loop the ambience AmbUWBA at full volume; every other story level SHALL stop the ambience loop.

#### Scenario: Fourth level
- **WHEN** exanima04 is played
- **THEN** Msc_UWB2_Amb plays with AmbUWBA

### Requirement: Story music change
Entering a level SHALL change the music only when its check's track, which names Msc_Countfour for byte 8 `7`, differs from the playing one in the first 15 characters.

#### Scenario: Same track
- **WHEN** exanima07 is entered while Msc_Countfour plays
- **THEN** the music does not change

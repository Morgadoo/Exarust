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

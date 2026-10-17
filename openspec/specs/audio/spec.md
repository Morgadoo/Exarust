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

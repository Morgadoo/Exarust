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

### Requirement: Voice ranking
The system SHALL keep 512 voices and, every update, rank the active ones by audibility (their gain below 30 m from the listener, gain × 4 m / distance beyond, their gain when placed relative to the listener) and bind the most audible, at most 64, to sources, freeing or deactivating the rest.

#### Scenario: More voices than sources
- **WHEN** 66 one-shots are active at once
- **THEN** 64 of them play and the two ranked lowest are freed

#### Scenario: A louder voice
- **WHEN** 64 one-shots play and a louder one starts
- **THEN** it takes the slot of the one that falls out of the top 64

### Requirement: Distance model
The system SHALL play every source at full gain within 4 m of the listener, falling linearly to silence at 30 m, held between the source's minimum gain and 1.

#### Scenario: Seventeen metres
- **WHEN** a one-shot at gain 0.8 plays 17 m from the listener
- **THEN** it is heard at gain 0.4

#### Scenario: Unattenuated
- **WHEN** a voice that defeats attenuation plays 90 m away at gain 0.3
- **THEN** it is heard at gain 0.3

### Requirement: Listener placement
The system SHALL place the listener 3 m along its up vector from an anchor 50 cm above the player.

#### Scenario: Upright listener
- **WHEN** the anchor is at (100, 50, 0) m and the up vector is +Y
- **THEN** the listener is at (100, 53, 0) m

### Requirement: Sound cache
The system SHALL load each sound once per name (the first 16 bytes, lower-cased) with a reference count, reuse the lowest free slot, and load a set's variants `base01` to `base16`, stopping at the first missing one, as buffers of the set's own.

#### Scenario: Release
- **WHEN** a set's own copy of `stpmed01` is released for the last time
- **THEN** the name `stpmed01` leaves the name index

### Requirement: Sound lengths
The system SHALL time a sound from its PCM header, counting a data byte as 1, 1/2 or 1/4 of a frame per frequency for mono 8-bit, mono 16-bit or stereo 8-bit, and stereo 16-bit, rounded to whole milliseconds.

#### Scenario: One second of stereo
- **WHEN** a stereo 16-bit file holds 176 400 bytes at 44.1 kHz
- **THEN** it lasts 1000 ms

### Requirement: Music crossfades
The system SHALL fade music over 3 s: a crossfade raises the new track linearly while the old one falls as (1 − t)² and then replaces it; an overlay ducks and pauses the main track and its fade-out resumes it; interrupting one fade with the other SHALL continue from the mirrored point.

#### Scenario: Halfway through a crossfade
- **WHEN** 1.5 s into a crossfade from a track at volume 1 to one at volume 0.5
- **THEN** both play at gain 0.25

#### Scenario: Interrupted fade-out
- **WHEN** an overlay fade-in is asked for 0.5 s into its fade-out
- **THEN** it continues as if 2.5 s of the fade-in had passed

### Requirement: Level music change
The system SHALL stop the main track and forget its name when a level changes, then play the level's track at once.

#### Scenario: New level
- **WHEN** a level with a different track is played
- **THEN** the old track stops and the new one starts without a crossfade

### Requirement: Music setting
The system SHALL stop the main track when music is switched off and play the remembered track again when it is switched on, only when the remembered volume is above 0.01.

#### Scenario: Quiet track
- **WHEN** music is switched off while the remembered volume is 0.01
- **THEN** the main track keeps playing

### Requirement: Area ambience
The system SHALL loop an area's bed `Env<area>01` at 0.15 × its volume from a random point in the first 0.8 s, and play its `Amb<area>` one-shots on the game clock: the first 15–45 s after the area is set, mono ones 10–20 m from the listener anchor and 5–15 s apart, stereo ones on the listener and 15–45 s apart.

#### Scenario: Outdoor arena
- **WHEN** an outdoor arena is played by day
- **THEN** the `NtrA` bed loops and its one-shots play around the listener

### Requirement: Reverb parameters
The system SHALL keep one global EAX reverb with the original's stored parameters and a decay of 1.49 s plus the level's decay offset, applied while the reverb is on.

#### Scenario: Level decay
- **WHEN** a level's decay offset is 0.51 s and the reverb is on
- **THEN** the reverb decays over 2 s

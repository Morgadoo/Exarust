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

### Requirement: Prop collision sounds
Each body tick, the game SHALL ring a prop's pair impacts in pair order, then its level impacts. An impact SHALL sound when it is above 0.5 and its part's clock has passed or it is above the part's threshold. The part SHALL play its material collision set as a one-shot at its mesh centre in the world. Held items and character bodies SHALL be left to their own sound paths.

#### Scenario: A crate dropped on the floor
- **WHEN** a wooden prop part of contact mobility 0.3 takes a level impact of 50 at game time 1000 ms with the generator at 1
- **THEN** its `wdlcol` set plays at volume 0.75 and pitch about 0.906, its threshold becomes 100 and its clock 1138 ms

#### Scenario: A second impact in the same tick
- **WHEN** the same part takes another impact of 60 in that tick
- **THEN** nothing more plays, since the clock holds and 60 is not above 100

### Requirement: Prop collision clock
A sounding part SHALL store twice the strength as its threshold and set its clock to `now + 220 − round(170·v)` plus a draw below 100 ms taken after the play. A part whose set has no shipped variant SHALL stay silent and park its clock at the all-ones word.

#### Scenario: A part without a shipped set
- **WHEN** a part whose collision set has no variant takes an impact of 50 past its clock
- **THEN** nothing plays, no draw is taken and its clock becomes the all-ones word

### Requirement: Scrape loops
Each scrape source of a body SHALL drive one looping voice: over the speed floor a voice that is not playing SHALL start on a variant of the source's set at volume and pitch 1, and the voice SHALL then take the source's volume, pitch and point; under the floor a playing voice SHALL be asked to stop. A body that leaves the world SHALL free its voices. A source whose sound has no shipped variant SHALL never bind a voice.

#### Scenario: A flail chain dragged
- **WHEN** a source's speed rises over the floor with volume 0.06 and pitch 1.0836
- **THEN** a looping voice starts and plays at gain 0.06 and pitch 1.0836 at the source's point, and gets the stop bit once the speed falls under the floor

### Requirement: Swing voice
The system SHALL play a fighter's swing on the voice of its first held item: restarted on a `SwingHA` variant, silent, at 0.4 × the primary striker's swing pitch on the first attack tick in the swing's region, and again only after leaving it. Every attack tick after the start in the region SHALL set its volume and pitch by the swing sound rule from the tracked speed. A fighter holding no item SHALL have no swing voice.

#### Scenario: A sword swing's voice
- **WHEN** a fighter holding a sword reaches 30 % of its attack clip
- **THEN** its voice starts silent at 0.4 × the sword's swing pitch and the same tick sets its volume and pitch from the tracker, with no second start in the region

#### Scenario: An unarmed swing
- **WHEN** an unarmed fighter attacks
- **THEN** no swing voice is cued

### Requirement: Swing voice fade and rustle
Every body tick that is not an attack SHALL fade a playing swing voice by 0.01 of volume and 0.0005 of pitch and stop it under 0.005. A swing voice's start SHALL rustle the fighter's clothing foley once its foley clock has passed, then hold the clock 400 ms plus a jitter under 100.

#### Scenario: The voice fades after the attack
- **WHEN** a fighter's attack ends while its swing voice still plays at volume 0.5
- **THEN** each following body tick lowers it by 0.01 until it stops under 0.005

### Requirement: Full voice pool
The system SHALL give the music and ambience callers a shared stand-in voice when all 512 voices are in use; writes to it SHALL never reach a source, and freeing it SHALL only clear the caller's holder.

#### Scenario: Main track with a full pool
- **WHEN** the main track is played while all 512 voices are allocated
- **THEN** its gain is written to the stand-in voice, no source is bound, and stopping the track leaves the pool untouched

### Requirement: Story level ambience
The system SHALL set a story level's ambience from the area its name's bytes pick at volume 1, and clear the ambience for a story level that picks none.

#### Scenario: Fourth level
- **WHEN** `exanima04` is loaded
- **THEN** the ambience area is `AmbUWBA` at volume 1, which names the bed `EnvAmbUWBA01` and the set `AmbAmbUWBA`

#### Scenario: Other story level
- **WHEN** `exanima02` is loaded
- **THEN** the ambience is cleared

### Requirement: Outdoor arena weather ambience
The system SHALL choose an outdoor arena's ambience in this order: rain above 0.01 clears it; daylight below 0.5 plays `NtrAN` at 0.3; a calm arena plays `NtrA` at 1; wind above 0.1 plays `NtrW` at 1 with bed gain `wind² × 0.2` and pitch `0.6 + 0.5 × wind`; otherwise it is cleared. Single values SHALL be compared to the double thresholds in double precision, and unordered values SHALL take the not-greater branch.

#### Scenario: Windy day
- **WHEN** daylight is 0.9, wind 0.5, no rain and the arena is not calm
- **THEN** `NtrW` plays with bed gain 0.05 and pitch 0.85

#### Scenario: Single-precision threshold
- **WHEN** the wind is the single value 0.1
- **THEN** it is above the double 0.1 and `NtrW` plays

### Requirement: Rain loop
The weather's first rain request SHALL take a voice whose flag word is listener-relative alone; each weather tick SHALL write the rain sound's volume and pitch into it, start the `rainfall` buffer at the listener, looping, when the tick asks for a start and the voice is not playing, and ask a playing voice to stop when the tick asks for a stop. The voice SHALL be released with the level's scene environment.

#### Scenario: Rain starts
- **WHEN** the tick asks for a start at volume 0.12 and pitch 1.3 and the voice is idle
- **THEN** the voice is listener-relative, allocated, restarting and looping at position zero with gain 0.12 and pitch 1.3

#### Scenario: Rain stops
- **WHEN** the tick asks for a stop while the voice plays
- **THEN** the voice's stop bit is set

### Requirement: Thunder
A thunderclap SHALL play one variant of the `Thunder` set as a listener-relative one-shot at the strike's position in metres, at the clap's gain and pitch.

#### Scenario: Strike ahead of a view off the origin
- **WHEN** a strike 300 cm out is moved 200 cm ahead of a view 10 m along x
- **THEN** the clap plays 15 m from the listener at gain 3.61

### Requirement: Arena weather ambience caller
Once an arena level's scene environment is in, the game SHALL set the ambience the arena setup chooses: none for the indoor arenas, and for the outdoor ones the weather rule over the scene's daylight, wind and rain target with the arena's calm flag (arenao1, arenao3 and arenao5 are calm), the wind bed's gain and pitch written over the bed after the area is set.

#### Scenario: Rainy arenao3
- **WHEN** arenao3's scene draws a rain target of 0.86
- **THEN** the ambience is cleared

#### Scenario: Windy arenao2
- **WHEN** arenao2's scene draws daylight 0.9, wind 0.6 and no rain
- **THEN** `NtrW` plays with bed gain 0.072 and pitch 0.9

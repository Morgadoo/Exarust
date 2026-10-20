## ADDED Requirements

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

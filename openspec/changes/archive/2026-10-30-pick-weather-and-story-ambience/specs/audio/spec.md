## ADDED Requirements

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

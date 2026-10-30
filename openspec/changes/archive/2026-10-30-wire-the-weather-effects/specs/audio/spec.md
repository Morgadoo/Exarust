# Spec Delta

## ADDED Requirements

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

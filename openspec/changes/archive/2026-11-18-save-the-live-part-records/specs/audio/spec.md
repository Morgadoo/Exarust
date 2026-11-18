## ADDED Requirements

### Requirement: Menu music crossfade
The main menu's music code SHALL crossfade the main track to the menu theme it selects (the drum-and-bass theme at 0.2, the plain one at 0.1) through the audio environment's crossfader.

#### Scenario: The music code typed
- **WHEN** the music code is typed on the main menu with the plain theme playing
- **THEN** a crossfade to the drum-and-bass theme at 0.2 is asked for

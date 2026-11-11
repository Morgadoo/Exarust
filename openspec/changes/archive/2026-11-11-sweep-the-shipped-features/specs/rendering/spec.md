## ADDED Requirements

### Requirement: Fires settle behind the held load
While a level's GI waits for its fires and the load holds game time, each fire whose flame has not burnt its two seconds SHALL run the flame ticks it still needs at once, the same count whatever the load's frames, so the GI builds behind the loading screen. A pause with no GI waiting SHALL hold the fires.

#### Scenario: The arena loads
- **WHEN** the arena, whose fires stand when its GI is asked for, loads with game time held
- **THEN** its fires settle in the load's first frames and the loading screen goes within seconds, not at the 60 s timeout

#### Scenario: A menu pause
- **WHEN** game time is paused with no GI waiting
- **THEN** no flame ticks

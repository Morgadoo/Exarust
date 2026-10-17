## ADDED Requirements

### Requirement: Death fade
When the player falling to its death hears the pit, the system SHALL cover the game's view with black, over the heartbeat's border and under the health bars, its opacity of 255 the share of 3 s of game time gone, rounded half to even, and full after; hidden while a level loads.

#### Scenario: Three seconds
- **WHEN** the fade has run 0, 12, 1,500 and 3,000 ms
- **THEN** its opacity is 0, 1, 128 and 255 of 255

#### Scenario: Paused mid-fade
- **WHEN** `exarust --at 0,200,0 --pause-at-tick 300 play arena` pauses 1.5 s into the fade
- **THEN** the falling body is drawn at about half its brightness, against the same body unfaded at tick 150

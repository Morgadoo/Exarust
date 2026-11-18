## ADDED Requirements

### Requirement: Game skin atlas at the GUI unit
The game skin's atlas SHALL be `guiskin.rfi` resampled by the engine's resample to an arbitrary size to 40 by 20 GUI units in the file's own format word and flags, once the game's data is open and again whenever the GUI unit changes.

#### Scenario: Unit 4 then 6
- **WHEN** the GUI unit is 4 and then becomes 6
- **THEN** the skin's atlas is 160 by 80 pixels and then 240 by 120

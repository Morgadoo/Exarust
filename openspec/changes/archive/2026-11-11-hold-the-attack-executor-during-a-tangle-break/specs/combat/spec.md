## ADDED Requirements

### Requirement: Tangle break holds the executor
The attack executor SHALL start nothing for a fighter whose swing does not stall while it plays Break Tangle, even once its cooldown has passed. A stalling fighter's released swing SHALL be judged without regard to the tangle break.

#### Scenario: Attack during a tangle break
- **WHEN** a player whose cooldown has passed requests an attack while its Break Tangle clip plays
- **THEN** nothing starts until the clip has returned to the stance

### Requirement: Silent main-side start
The system SHALL play no sound when a main-side attack starts, because the original's body set-up sets the quiet byte that guards that sound for every character and nothing clears it.

#### Scenario: Main-side start
- **WHEN** a fighter starts a main-side attack from rest
- **THEN** no start sound plays at its hip

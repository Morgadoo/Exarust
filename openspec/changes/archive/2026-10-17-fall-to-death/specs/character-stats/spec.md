## ADDED Requirements

### Requirement: Falling to death
Each tick a living body in the falls branch SHALL run the fall check: at −5 cm a tick or slower (its chest's slow vertical velocity) it is steady; faster, it screams once (not if dead); then lost if its pelvis is 2,500 cm or more over the ground under it and under each foot. More than 1,000 ms after (game clock) the player hears the pit and its screen fades; it dies after 4,000 ms, anyone else after 3,000: dead, health 0, out of the world.

#### Scenario: Thresholds
- **WHEN** a body sinks at 5 cm a tick, then 5.01, then 4
- **THEN** it is steady, screams once, then is steady and screams again the next time

#### Scenario: Twenty-five metres
- **WHEN** a falling body's pelvis is 25 m over the ground under it and its feet, then 24.99 m, or one foot is over ground 2 m down
- **THEN** it is lost only in the first case

#### Scenario: Timelines
- **WHEN** the player and another character are lost
- **THEN** the player hears the pit on the 84th tick after (1,008 ms) and dies on the 334th; the other dies on the 251st, hearing nothing

#### Scenario: Dropped from 200 m
- **WHEN** `exarust --at 0,200,0 play arena` drops the player
- **THEN** it screams and is lost on the same tick, the pit sounds 84 ticks later and it is gone 334 ticks after the loss

### Requirement: Kill plane
A dead character's body whose root falls below −50 m SHALL lose its health and, unless it is the player's, leave the world with everything it carries; once. A living body, or one already lost to its fall, SHALL not be taken.

#### Scenario: A corpse falls off the world
- **WHEN** a dead character's body, the dead player's and a living body fall below −50 m
- **THEN** the corpse is gone, the player's body stays with no health, and the living body is left alone

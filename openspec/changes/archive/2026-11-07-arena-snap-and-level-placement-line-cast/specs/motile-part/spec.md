## ADDED Requirements

### Requirement: Arena spawner snap
A body the arena's spawners stand (a generated or hire opponent, the Beast, a company entrant) SHALL be stood 5 cm over the point the line cast meets from its place lifted 10 m, straight down at most 30 m; a miss SHALL keep its place. In an elimination (7) or reserve (5) match the player's spot SHALL be snapped the same way; a duel's and challenger's spot SHALL not.

#### Scenario: A raised floor
- **WHEN** an opponent is planned at 5 cm and the cast meets a floor at 44 cm
- **THEN** it stands at 49 cm

#### Scenario: Duel
- **WHEN** the player enters a duel
- **THEN** its spot is not cast

### Requirement: Placement on a loaded level
When the player is placed on a loaded level (a save load or a level change), the floor SHALL be probed with the line cast from 100 cm over the placement, 300 cm down, and the feet SHALL stand 5 cm over the hit; a miss SHALL keep the placement.

#### Scenario: Probe
- **WHEN** the placement is at y 0 and the cast meets a floor at 30 cm
- **THEN** the feet stand at 35 cm

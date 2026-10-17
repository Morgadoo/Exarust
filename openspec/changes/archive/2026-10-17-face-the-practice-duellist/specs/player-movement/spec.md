## MODIFIED Requirements

### Requirement: Original starts
The system SHALL start the player where the original does for a new game in the first story level, the arena hub and arena matches, facing as the original does; in an arena duel the opponent SHALL stand opposite, both facing the arena's centre. In a level without an arena ring (the practice arenas) the opponent SHALL stand 2.5 m in front of the player, facing it.

#### Scenario: Arena duel
- **WHEN** `exarust --opponent play arenao3` runs
- **THEN** the player stands 2 m from the arena centre along +Z facing it, and the opponent 2 m along −Z facing it

#### Scenario: Practice duel
- **WHEN** `exarust --opponent play arena` runs
- **THEN** the player stands at (0, 0, 5) facing −Z and the opponent at (0, 0, 2.5) facing it, so the duellist's blows meet the player's front and land

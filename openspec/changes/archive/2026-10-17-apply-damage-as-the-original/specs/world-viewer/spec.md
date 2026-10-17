## ADDED Requirements

### Requirement: Feral player
In play mode `--feral f` SHALL give the player's record the feral level f (0 to 1), so the player spawns with a feral character's stats and rule bits.

#### Scenario: Feral duel
- **WHEN** `exarust --feral 0.5 --opponent play arena` runs and the duellist's blows land
- **THEN** the player's red health stays 0.25 while its yellow falls, and its energy recovers to at most 0.95

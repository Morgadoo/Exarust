# Spec Delta

## ADDED Requirements

### Requirement: Live discovery pass
In the running story, each of the flow's once-a-second discovery passes SHALL convert the account pool, the player and each companion at the game clock, and SHALL write the player's remaining experience, skills and powers back to the player's record.

#### Scenario: One pass after the level start
- **WHEN** a player carrying 600 experience, training its first skill from 5000 points, starts at 0 and a pass runs at 1000
- **THEN** the record holds 580 experience and the first skill 5008 points

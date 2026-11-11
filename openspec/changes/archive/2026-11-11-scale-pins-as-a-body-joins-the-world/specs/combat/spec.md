## ADDED Requirements

### Requirement: Player target before the attack starters
The player's combat target SHALL be picked by its behaviour's auto-target in each fixed step before the attack starters aim, so an attack started in that step aims at that step's pick. Out of combat mode the target SHALL be none. The behaviour's steering SHALL keep reading the target stored with the stance.

#### Scenario: Opponent ahead
- **WHEN** the player in combat mode has an opponent 2 m ahead in the cursor's direction
- **THEN** its combat target is that opponent in the same step

#### Scenario: Combat mode off
- **WHEN** the player leaves combat mode
- **THEN** its combat target is none in that step

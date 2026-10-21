# Spec Delta

## MODIFIED Requirements

### Requirement: Still opponent
`--still-opponent` SHALL spawn the play mode duellist as `--opponent` does, standing its ground: it SHALL face the player and take the stance its combat state chooses, but neither walk nor strike.

#### Scenario: Stances at a distance
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs
- **THEN** the duellist stays 8 m from the player, facing it in Cmbt - Idle while it is fresh

#### Scenario: Unattended attacks
- **WHEN** `exarust --opponent --weapon ub05ssworda --press LMB@100-1400 play arena1` runs
- **THEN** the player, in combat mode from the fight start, swings its sword from frame 100, and again 426 ms after each swing's last tick while the button is held

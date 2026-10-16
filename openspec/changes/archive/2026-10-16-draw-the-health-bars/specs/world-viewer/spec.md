## ADDED Requirements

### Requirement: Scripted health
In play mode `--health yellow,red` SHALL give the player that starting yellow and red health, and `--heartbeat` SHALL let the low-health heartbeat play in any level.

#### Scenario: Retail values
- **WHEN** `exarust --health 0.176,0.226 --press Tab@50 play arena` runs
- **THEN** from frame 50 the health bar shows the yellow, brown and red shares of the retail reference

#### Scenario: Heartbeat in the arena
- **WHEN** `exarust --heartbeat --opponent play arena` runs and the duellist's blow leaves the player's yellow health below 0.05
- **THEN** the heartbeat's border pulses for 7 s

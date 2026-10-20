# Spec Delta

## ADDED Requirements

### Requirement: Grid look-ahead
The player's look-ahead SHALL walk the navigation grid from the standing point toward the look-ahead goal (75 cm + 6·v², 70 cm when backing) and SHALL stop the character when the walk is blocked within 100 cm, as the original does.

#### Scenario: Walking into a crate
- **WHEN** the player walks toward a crate lower than its hip
- **THEN** the player stops in front of it instead of pushing into it

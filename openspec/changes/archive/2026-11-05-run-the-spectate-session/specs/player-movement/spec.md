## ADDED Requirements

### Requirement: Spectate session
The controls' spectate session SHALL, at its start, set the spectate flag and free the camera; while it runs the player SHALL not be steered, and a double click SHALL make the camera follow the character picked under the cursor, or free it when the pick is anything else or nothing; a free camera SHALL keep looking where it last looked. Its end SHALL clear the flag and the follow. The play flag `--spectate` SHALL start it once the load is over.

#### Scenario: Following the opponent
- **WHEN** `play arena --opponent --spectate` runs and the opponent is double clicked
- **THEN** the camera follows the opponent and the player stands where it is

#### Scenario: A miss
- **WHEN** the double click picks the floor or nothing
- **THEN** the camera follows nothing and keeps its last point

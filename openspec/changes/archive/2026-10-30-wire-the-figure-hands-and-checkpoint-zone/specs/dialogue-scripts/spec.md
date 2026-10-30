# Spec Delta

## ADDED Requirements

### Requirement: CheckpointSave call
`CheckpointSave(n)` SHALL read its argument and drop the value, and SHALL report whether the script's first actor slot is the player; it SHALL answer 1 when that slot is filled, 0 otherwise.

#### Scenario: The zone's routine
- **WHEN** `CheckpointSave(1);` runs with the player in the first slot, then with a guard
- **THEN** the first reports the player and the second does not

### Requirement: Zone routine
A script zone's routine SHALL run with the entering character as the one actor slot, no role, and the local `ZoneID` holding the zone's id.

#### Scenario: A routine reading its zone
- **WHEN** `? ZoneID = 4 : { GameEvent(A1); } / { GameEvent(B2); };` runs for zone 4 and for zone 3
- **THEN** the first raises event 0xA1 and the second 0xB2

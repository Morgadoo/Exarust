## ADDED Requirements

### Requirement: The revive reset runs where the grid is
The story revive's brain reset SHALL run at the start of the next fixed step, in the world that holds the navigation grid and the footprints, so that its roam plans and its nearest-walkable query see the grid.

#### Scenario: A level with roaming brains
- **WHEN** the player is knocked out and revived in exanima08
- **THEN** the reset moves its roaming brains along their plans

### Requirement: The revive reset's footprint and rooms
A roaming reset SHALL lift the body's footprint before it plans and stamp it again where the body ends, and before each round it SHALL move the brain into the room of the point that round plans from.

#### Scenario: A roam with no room to plan in
- **WHEN** a roaming brain whose footprint is stamped is reset where no plan can be drawn, beside a brain that does not roam
- **THEN** both footprints end where they were and the grid's costs under them are unchanged

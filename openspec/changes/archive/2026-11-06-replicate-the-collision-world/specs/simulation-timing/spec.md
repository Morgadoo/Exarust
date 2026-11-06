## ADDED Requirements

### Requirement: Collision replica for frame picks
The frame's picks SHALL query a replica of the simulation's collision world that has the same collider handles and gives the same pick results as the simulation's world after the last finished step, through the frame's inserts and removals and the step's moves and removals.

#### Scenario: Picks after moves and removals
- **WHEN** the frame inserts and removes colliders and a step moves and removes others
- **THEN** every pick on the replica after the step returns the same collider and distance as on the simulation's world

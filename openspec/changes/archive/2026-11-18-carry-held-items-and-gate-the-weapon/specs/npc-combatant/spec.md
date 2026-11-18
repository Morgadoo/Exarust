## ADDED Requirements

### Requirement: Downed route stop
A downed body's AI tick SHALL stop its route every tick: the route ends, and on the body the run bit and the free walk clear.

#### Scenario: A runner knocked down
- **WHEN** an NPC running a route with its run bit and free walk set is downed
- **THEN** after its next AI tick the route is not followed and neither the run bit nor the free walk is set

### Requirement: Footprint overflow
An NPC whose footprint stamp falls within five cells of the navigation grid's edge on either axis SHALL stamp nothing and be removed from the scene once, the first overflow of the run logged as "Character position overflow."; the player SHALL stay.

#### Scenario: An NPC at the grid's edge
- **WHEN** an NPC stands at cell 36 of a 40-cell grid and its footprint is kept
- **THEN** its body is despawned and no footprint of it is on the grid

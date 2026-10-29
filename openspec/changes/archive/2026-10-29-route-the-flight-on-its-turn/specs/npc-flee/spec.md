## ADDED Requirements

### Requirement: The routed flight
On a level with a navigation grid a fleeing brain SHALL choose and route its way out only on its planning turn (one planner a frame) and SHALL choose again while no route is followed; it SHALL follow a route only when its ends lie more than 150 cm apart, steering toward the route's carrot, and SHALL remember the exit's room when a followed route ends in its own room within 15 m of the threat.

#### Scenario: Routed on its turn
- **WHEN** a brain at level 4 flees from the first of two rooms on a level with a walkable navigation grid
- **THEN** its first frame only records its wait, the next frame picks the way out and follows a grid route ending within 50 cm of it, and it jogs along the route

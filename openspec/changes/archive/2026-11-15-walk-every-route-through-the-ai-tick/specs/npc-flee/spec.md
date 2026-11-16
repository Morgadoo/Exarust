## MODIFIED Requirements

### Requirement: The routed flight
On a level with a navigation grid a fleeing brain SHALL choose and route its way out only on its planning turn (one planner a frame) and SHALL choose again while no grid route is followed; a route whose ends lie more than 150 cm apart SHALL become the character's route at urgency 2, walked by the AI's walk, and the exit's room SHALL be remembered when that route ends in the brain's room within 15 m of the threat.

#### Scenario: Routed on its turn
- **WHEN** a brain at level 4 flees from the first of two rooms on a level with a walkable navigation grid
- **THEN** its first frame only records its wait, the next frame picks the way out and its route follows a grid path ending within 50 cm of it at urgency 2, and it runs along the route

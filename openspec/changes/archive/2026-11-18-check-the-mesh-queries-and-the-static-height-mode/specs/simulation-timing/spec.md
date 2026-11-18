## ADDED Requirements

### Requirement: Scene grid reset at the teardown
The level teardown SHALL put the scene grid back to its default and empty the static collider grid, standing in for the grid rebuild the original's scene clear asks of the simulation.

#### Scenario: A teardown after an all-pairs scene
- **WHEN** a level whose scene had no grid is torn down
- **THEN** the scene grid is the default again

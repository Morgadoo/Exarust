## ADDED Requirements

### Requirement: Installed pathfinder
The system SHALL answer pathfinder queries through the level's grid once it is built, and through a 64 x 64 abstract pathfinder while the grid is being built or when the level has none, the abstract pathfinder answering as if there were nothing there: no obstacle, no clamped line, no snapping, no blocked point, no reach, the point unchanged.

#### Scenario: Level without a grid
- **WHEN** a level has no navigation grid
- **THEN** the look-ahead is never blocked and a snap leaves the point as it was

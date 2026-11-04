# Spec Delta

## ADDED Requirements

### Requirement: Door passage report
A door's passage SHALL NOT change any cell of the navigation grid: the original reports it to a pathfinder slot that does nothing, so a door shut at the build keeps its doorway raised once opened until a refresh covers the doorway, and the room route alone follows the door.

#### Scenario: A door toggled open
- **WHEN** exanima01's start door is toggled open and swings to rest without `--nav-refresh-moved`
- **THEN** the grid is not refreshed and a path from the player through the doorway still ends short

### Requirement: Grid revision
The system SHALL count the grid's builds and the refreshes that change a floor (`NavRevision`), and the debug path SHALL plan again when the count changes.

#### Scenario: A rested leaf with the debug option
- **WHEN** with `--nav-refresh-moved` exanima01's start door is toggled open and comes to rest
- **THEN** a refresh raises the revision and the debug path, planned again, reaches a point beyond the door

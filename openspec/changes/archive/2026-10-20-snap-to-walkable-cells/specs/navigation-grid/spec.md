## ADDED Requirements

### Requirement: Walkable snap
The snap SHALL return the start cell when it has a floor and its A and B bytes sum under 80, with height base + floor·10. Otherwise it SHALL sweep the 32×32 window in distance order, up to 220 nodes, and return the nearest walkable cell or nothing.

#### Scenario: Walkable, nearby and none
- **WHEN** the start cell is walkable
- **THEN** it is returned at once with the snapped height
- **WHEN** the start is blocked and one cell in the window is walkable
- **THEN** the sweep returns that cell
- **WHEN** every cell in the window is blocked
- **THEN** the snap returns nothing within the node cap

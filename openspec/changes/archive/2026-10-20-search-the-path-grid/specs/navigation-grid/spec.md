## ADDED Requirements

### Requirement: Path search
The search SHALL pop a LIFO open list by the smallest f (newest wins a tie), stop at the acceptance distance, an empty list or 4095 nodes, re-parent an open neighbour to a smaller f, and rebuild the path as the parent chain collapsed through collinear runs. It SHALL count the goal as reached only when the best node is the goal cell.

#### Scenario: Straight, detour and sealed
- **WHEN** a clear five-by-five grid is searched between (0, 2) and (4, 2)
- **THEN** the path runs straight along y = 2 from the start to the goal and the goal is reached
- **WHEN** a wall blocks the direct row
- **THEN** the path detours around it and every point is walkable
- **WHEN** the goal is sealed by all its in-bounds neighbours
- **THEN** the search reports the goal as not reached

# Spec Delta

## ADDED Requirements

### Requirement: Straight walk
A straight walk SHALL allow a floor five above the current height and SHALL block one six above when the underside is within fifteen. After a lower floor the height SHALL become that floor, so a later rise is measured from it.

#### Scenario: Five is allowed, and a drop is remembered
- **WHEN** a floor rises by five, the same floor rises by six, and a floor drops by four before rising by six
- **THEN** the rise of five is open while the line walk blocks it, the rise of six blocks, and the drop makes the later rise block

### Requirement: Obstacle
A line of 40 cm or less SHALL not be an obstacle. A longer line SHALL be one when a floor is more than seven too high, or when a cell before the last two has a penalty above 60. A penalty of 60, and a penalty above 60 on a last cell, SHALL not.

#### Scenario: Forty centimetres is clear, sixty-one is not
- **WHEN** a high floor is one cell ahead, the same floor is two cells ahead, a penalty of 61 is on an early cell, and a penalty of 60 is early while 61 is on the last cell
- **THEN** one cell ahead is not an obstacle, two cells ahead is, the early 61 is, and the 60 with a late 61 is not

### Requirement: Nearest walkable
A cell whose clearance and penalty sum to less than 80 SHALL keep the point and take the floor height. A cell at 80 SHALL step to the first cheaper neighbour centre.

#### Scenario: Under 80 stays, 80 steps aside
- **WHEN** a point sits off the centre of a cheap cell, and a point sits on a cell whose penalty is 80
- **THEN** the cheap point keeps its place at the floor height, and the costly point moves to the centre of the neighbour one cell toward the origin

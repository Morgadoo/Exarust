# Spec Delta

## ADDED Requirements

### Requirement: Look-ahead
The look-ahead SHALL be 75 cm + 6·v² forward, 45 cm + 6·v² in combat, and 70 cm backward, where v is the speed along the heading in centimetres a tick.

#### Scenario: Standing and walking
- **WHEN** the speed is 0, then 2 cm a tick
- **THEN** the forward reach is 75 cm, then 99 cm, and the combat reach at 2 cm a tick is 69 cm

#### Scenario: Backing
- **WHEN** the character backs up
- **THEN** the look-ahead is 70 cm behind the facing

### Requirement: Path around a wall
A search on the grid SHALL return cell-centre points from the start cell to the goal cell that do not enter a blocked cell. When a wall sits between them, the path SHALL be longer than the straight line.

#### Scenario: A wall across the straight line
- **WHEN** a flat grid has a wall between two walkable cells and a gap at the wall's end
- **THEN** the search reaches the goal, no point of the path is on the wall, and the path is longer than the straight line

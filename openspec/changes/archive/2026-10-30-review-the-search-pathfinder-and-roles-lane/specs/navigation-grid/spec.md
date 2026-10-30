# Spec Delta

## MODIFIED Requirements

### Requirement: Line walk
A line walk SHALL step from the cell after the start along the longer axis, Z when the two spans are equal, one step per cell of the major span up to and including the asked end's cell. A floor more than four above the current height, whose underside is within ten, SHALL block. The first blocked step SHALL keep the start. A later one SHALL stop before that cell and keep the asked height. A clear line SHALL keep the asked end.

#### Scenario: An open floor, then a floor too high
- **WHEN** a line crosses an open floor from cell 1 to cell 6, the same line meets a floor of 40 at cell 4, and a line starts on the cell before that floor
- **THEN** the open line keeps its end, the walled line blocks before cell 4 and keeps the asked height, and the line that starts against the floor keeps its start

#### Scenario: The end cell is walked
- **WHEN** a line from cell 1 to cell 6 meets a floor of 40 at cell 7, then also at cell 6
- **THEN** the line, the straight walk and the obstacle test are clear, then all three block

#### Scenario: A diagonal walks along Z
- **WHEN** a line runs from cell (1, 1) to cell (4, 4)
- **THEN** its major axis is Z and it takes three steps

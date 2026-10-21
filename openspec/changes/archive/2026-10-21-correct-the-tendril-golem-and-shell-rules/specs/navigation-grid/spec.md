# Spec Delta

## ADDED Requirements

### Requirement: Line stamp
Stamping a line SHALL weigh each line point's cell 255, the eight cells around each line cell 128 (255 when reached a second time), and the cells beside those that neither pass weighed 64 more each time they are reached, at most 255. Each weighed cell SHALL add a quarter of its weight to its layer's penalty byte, wrapping, and taking the stamp back SHALL subtract the same.

#### Scenario: A two-cell line
- **WHEN** a line runs through two neighbouring cells on the lower layer
- **THEN** the line cells gain 63, a cell beside both gains 63, a corner of one gains 32, and taking the stamp back leaves every penalty as it was

### Requirement: Line stamp layer
A line cell SHALL be weighed on the layer whose height band holds its point, the lower layer first, and the cells around the line on the layer that holds the first point's height. A height band SHALL hold a height, in 10 cm steps over the grid's base, more than three over the cell's underside and under its floor plus 25.

#### Scenario: An upper floor
- **WHEN** a point stands 96 steps up over a cell whose lower floor is at 10 and whose upper floor spans 90 to 100
- **THEN** its cell is weighed on the upper layer

### Requirement: Barricade stamp in play
A standing barricade's line SHALL be stamped on the navigation grid and taken back when its fall ends.

#### Scenario: A barricade on a grid
- **WHEN** a barricade stands on a grid and then falls
- **THEN** its line raises the grid's penalty while it stands and the penalty returns to zero after the fall

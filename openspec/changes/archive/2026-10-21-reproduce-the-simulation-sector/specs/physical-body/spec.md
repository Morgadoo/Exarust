# Spec Delta

## ADDED Requirements

### Requirement: Simulation sector grid
The system SHALL bucket simulated bodies in the original's 600 cm grid: a point's cell is the round-down of its x and z in centimetres times the single-precision 1/600, a NaN or unrepresentable value giving the most negative integer, and a body's cell is that of its mesh box's world centre. A body's cell index SHALL count as overflowing in the first row or at or past the last row; a sim object's also on the first cell of the second row.

#### Scenario: Cells
- **WHEN** points at 599.9, 600, −0.1 and −600.1 cm are bucketed
- **THEN** their cells are 0, 1, −1 and −2, and NaN gives the most negative integer

### Requirement: Static collider span
A static collider's span SHALL round its lower and upper edges half to even, floor the lower at cell 1 and cap the upper at the last cell but one; when either edge leaves the centre's cell it SHALL be inserted into every cell from one below the lower edge to one above the upper edge on both axes, the capacity the original sizes for, and otherwise into its centre's cell.

#### Scenario: A wide static bound
- **WHEN** a bound centred at 1500 cm with a 700 cm half extent is inserted in an 8 × 6 grid
- **THEN** its edges are cells 1 and 4 and it goes into the 36 cells 0 to 5 on both axes

#### Scenario: Half a cell
- **WHEN** a bound at the origin has a 300 cm half extent
- **THEN** both its edges round to the even cell 0 and it is not wide; at 301 cm it reaches cells −1 and 1

### Requirement: Pair meet test
The system SHALL gate a pair of body parts as the original does: the awake part's sphere bound must touch the other's mesh sphere (touching counts); then the more mobile part's sphere bound must meet the other's mesh box, and the other's mesh bound its sphere box. A bound meets a box when, carried over the tick in the other part's frames, it enters the box grown by its radius on every axis before the end of the travel and, from 0, strictly before the exit.

#### Scenario: A fast pass
- **WHEN** a 10 cm sphere crosses a 10 cm box from 2 m before it to 2 m past it within one tick
- **THEN** the meet test passes although both ends miss the box

#### Scenario: A box's corner
- **WHEN** a resting 10 cm sphere lies 11.3 cm from a box's corner, inside the box grown by 10 cm
- **THEN** the meet test passes

#### Scenario: Mesh boxes apart
- **WHEN** two balls' spheres overlap but their mesh boxes are 1 cm across
- **THEN** no contact is generated

### Requirement: Body bounds for pairs
A part's sphere bound SHALL be the box grown from its first sphere's centre to its spheres' extremes, centred at low + (high − low)/2, with the largest distance plus radius as its radius (a sphere within the original's epsilon of the centre counting its radius alone). A prop's mesh bound SHALL be its node's box and bounding radius. In the all-pairs walk of a small sector each part's sphere centre SHALL also lie within its sphere radius of the other's mesh box.

#### Scenario: Two spheres
- **WHEN** spheres of 10 and 20 cm sit 100 cm apart
- **THEN** the bound is centred 55 cm from the first, 65 by 20 by 20 cm in half extents, with a 65 cm radius

### Requirement: Sector overlap tests
The system SHALL provide the original's overlap-only tests: the body test measures a point's overhang out of a box by the squared overhang of the last axis it sticks out on, not their sum; the placement pre-test grows each sphere by 2 cm and asks it to lie within the other's bounding radius of its mesh centre and touch its mesh box; the static-collider pre-test asks the sphere to touch the mesh box and lie within the two radii of the mesh centre.

#### Scenario: Last overhang
- **WHEN** a 6 cm sphere's centre sticks 5 cm out of a box on x and on y
- **THEN** the overlap-only test counts 25 cm² and the sphere overlaps the box

## MODIFIED Requirements

### Requirement: Volume cut
The system SHALL cut a convex volume by a plane, keeping each face's part on the inner side, and SHALL chain the cuts into a cap face on the plane, closing it when its ends' squared distance is below 1e-4 (compared in double) and extending it from its start otherwise, a NaN distance included.

#### Scenario: A cube cut through its centre
- **WHEN** a cube of half side 10 cm is cut by x ≤ 0
- **THEN** it has the −X face, four halved faces and a square cap on x = 0

### Requirement: Directional shadow volume
The system SHALL build the volume a directional light's casters can lie in from a view volume: a side plane through each first shared edge (squared length (x² + y²) + z² not below 1 cm², a NaN length kept) between a face open to the light and one turned away, its polygon extruded 10 000 cm toward the light, followed by the faces turned away; it SHALL return the side planes' last index as their count.

#### Scenario: A cube lit from above
- **WHEN** a cube of half side 10 cm is given the light direction (0, −1, 0)
- **THEN** it gets four side planes, one per side face, the bottom face closes it, and the returned count is 3

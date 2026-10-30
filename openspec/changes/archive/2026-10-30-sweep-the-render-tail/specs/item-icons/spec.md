# Spec Delta

## MODIFIED Requirements

### Requirement: Scene fit distance
The icon camera SHALL stand on +Z from the item box's centre at the engine's scene-fit distance: the first cross half extent unless it is below the second (then the second, with the depth weight halved), bent toward the framing distance F while F is above it by `F·((r·0.11 + 1) − 1/(8r + 1))` with `r = h/F`, divided by the runtime's `sin(fov/2)` with the icon view's 40° field, plus the depth half times its weight.

#### Scenario: A small item at the default framing
- **WHEN** the half extents are (5, 2, 1) cm and the framing is 70 cm
- **THEN** the distance is 77.0322 cm

#### Scenario: The second half wins
- **WHEN** the half extents are (2, 50, 10) cm and the framing is 30 cm
- **THEN** the distance is 151.1902 cm

## ADDED Requirements

### Requirement: Scene fit node
The engine's scene fit SHALL put the node it moves at the box's centre plus the direction from that node's old position to the view node, normalised by the engine's normalise, times the fit distance.

#### Scenario: A node at the view node
- **WHEN** the moved node sits where the view node is
- **THEN** it lands on the box's centre

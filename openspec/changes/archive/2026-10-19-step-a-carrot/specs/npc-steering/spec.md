# Spec Delta

## ADDED Requirements

### Requirement: Path carrot
A ground point SHALL project onto the nearest path segment and keep the unclamped fraction. The goal SHALL be 100 cm further along that path, measured from the unclamped foot. An equal distance SHALL keep the earlier segment. A vertical segment SHALL be skipped. Exactly 100 cm left SHALL spill, and a zero step SHALL have no goal.

#### Scenario: One hundred centimetres past the foot
- **WHEN** a ground point 50 cm behind a 200 cm segment is projected, a point on a corner is projected onto a path that then rises, a point at the top of a vertical segment is projected, and a point at the start of a 100 cm segment with another 100 cm after it is projected
- **THEN** the foot behind is fraction -0.25 and the goal is 50 cm along, the corner keeps the first segment and the goal is 20 cm up and 100 cm along the next, the vertical segment is skipped and the goal is the last point, and the exact 100 cm has no goal while its cursor stays at the start

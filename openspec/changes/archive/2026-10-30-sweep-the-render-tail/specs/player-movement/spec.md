# Spec Delta

## ADDED Requirements

### Requirement: Free camera movement rules
The free camera's key vector SHALL subtract and add the right and forward axes in A, D, S, W order and add the controller's movement vector, then, when asked, zero the height and normalise a squared length above 2⁻²³; its tether SHALL hold an axis past its limit at `sign(offset)·limit + anchor` or scale a radial offset onto the radius keeping the height; its drift SHALL carry 0.96 of the change and keep 0.99 while the change squared exceeds 1e-7 or the velocity squared 1e-4 in double.

#### Scenario: The tether's sign
- **WHEN** the looked-at point is 410 cm behind its anchor on z with a 50 cm limit
- **THEN** it is held 50 cm behind the anchor

### Requirement: Free camera scene walks
The lift SHALL visit every sector's meshes whose class word holds 0x89 and whose flags word holds 0x200000, skipping those whose bounding sphere misses the point's ball or whose oriented box is out of reach; the wall probe SHALL visit only visible sectors and skip owned, blocking and unflagged meshes and those whose bounding sphere the segment misses.

#### Scenario: A hidden sector
- **WHEN** a lift and a wall probe meet a mesh in a sector whose visible bit is clear
- **THEN** the lift asks the mesh and the wall probe does not

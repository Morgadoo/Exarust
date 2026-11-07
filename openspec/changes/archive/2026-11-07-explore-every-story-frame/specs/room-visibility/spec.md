## ADDED Requirements

### Requirement: Placement pick behind an occluder
In a grid level, the item cursor SHALL place nothing in a frame whose pick hit lies past an active room's occluder along the cursor ray: no placed point and no placement marker.

#### Scenario: A shelf across a wall's hollow
- **WHEN** the cursor holding an item points at a surface seen across an active room's occluder
- **THEN** the frame places nothing and the item stays at its last placement

## ADDED Requirements

### Requirement: Floor texture mapping
The system SHALL map tile-grid floors as the original does. Every `+`-material triangle of a cell's floor pieces (centre variant, edge strips and the pieces' `+F` strips) gets planar UVs u = x/6 + ½, v = ½ − z/6 in cell-local metres, after the piece's rotation. That is one square texture repeat per cell, aligned to its borders. Walls, ceilings and other materials keep their stored UVs.

#### Scenario: Menu floor
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** the floor's bricks are the same size as in the original

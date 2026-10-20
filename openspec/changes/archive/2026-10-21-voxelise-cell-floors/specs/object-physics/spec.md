# Spec Delta

## ADDED Requirements

### Requirement: Rigid-body voxel map
The system SHALL keep the original's voxel map for a rigid-body mesh: made at a resolution of 0.02 with a unit scale and the caller's tag, a box cut into cells with one empty border cell on each side, one word per cell running Z fastest, two fill bytes per cell and a list of extracted spheres. Releasing the map's arrays SHALL free the cells and fill bytes and keep the spheres.

#### Scenario: Border cells
- **WHEN** a map is sized to 2 × 3 × 4 cells inside its box
- **THEN** it holds 4 × 5 × 6 cells and its first inside cell's centre is half a cell in from the box's low corner

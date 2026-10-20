# Spec Delta

## ADDED Requirements

### Requirement: Grid floors in the GI volume
The system SHALL give each tile-grid cell's floor the original's floor heightfield points: the cell's floor meshes are merged and sampled once, in the cell's space and in centimetres, with 10 cm columns cast straight down, the highest upward face marking the 10 cm cell under it, and a point at the centre of every 20 cm cell whose four columns all mark inside it. The points SHALL be made when the level's GI inputs are collected and splatted into every block built afterwards.

#### Scenario: Flat cell
- **WHEN** a 6 m cell's floor is flat at height 0
- **THEN** it gives 30 × 30 points 10 cm under the floor, on the 20 cm lattice around the cell centre

#### Scenario: Floor made of several meshes
- **WHEN** a cell's floor is split between two meshes inside a 20 cm cell
- **THEN** the merged floor still gives that cell's point, as one mesh would

#### Scenario: Hole in the floor
- **WHEN** a cell's floor has a 2 m square hole
- **THEN** the 20 cm cells over the hole get no point

### Requirement: Generated surface points
The system SHALL generate surface points for a mesh without stored GI points as the original does: cells of a fifth of the generator's parameter around the mesh's bounds are classified as on the surface or near it by two sphere queries, near cells beside an on cell are dropped, and a greedy 5 × 5 × 5 cover (or one point per on cell) turns the rest into points, sparse cells first and the first of equal windows winning.

#### Scenario: Flat slab
- **WHEN** the generator runs at its parameter of 20 on a 20 cm box crossed by a flat surface
- **THEN** the greedy cover gives four points on the surface, a quarter in from each corner

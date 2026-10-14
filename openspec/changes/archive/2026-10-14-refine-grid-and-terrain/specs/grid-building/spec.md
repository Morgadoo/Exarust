# Spec Delta

## ADDED Requirements

### Requirement: Piece strips and occluders
The system SHALL use a wall, corner or special-tile piece's own `+F` child as that edge's floor strip (with the floor set's surface material) and its `+R` child as the ceiling strip in place of the sets' strips, and SHALL never draw `[` occluder children.

#### Scenario: Wall with its own floor strip
- **WHEN** a wall piece with a `+F` child is placed on a cell edge
- **THEN** that strip is drawn with the floor set's material and no floor-set strip is added on that edge

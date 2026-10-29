## MODIFIED Requirements

### Requirement: Scene bounds
The system SHALL write the scene's bounds into its own sector: every other sector's boxes SHALL be turned into world-axis extents by that sector's rotation, summing each axis's three absolute matrix terms in order; its first box starts as its own first box and grows by every other sector's first box about its world centre (extents capped at its radius); the union corners start as its own second box's centre and half extents taken as corners and grow by every other sector's second box about its unmoved centre; its second box is left unchanged. The active-sector test SHALL meet a sector's box along the same rotation.

#### Scenario: One room
- **WHEN** a scene with empty bounds has one room whose boxes are centred at x = 6 m with extents (3, 1, 1) m
- **THEN** the scene box is centred at x = 4.5 m with extents (4.5, 1, 1) m and the union radius is √(4.5² + 1 + 1) m
- **WHEN** the scene's own second box is centred at x = 1 m with a 20 m x extent
- **THEN** the union radius is √(15.5² + 1 + 1) m

#### Scenario: A turned room
- **WHEN** a room at x = 10 m whose boxes have half extents (3, 1, 1) m is turned a quarter about the vertical
- **THEN** the scene box runs 0…11 m in x and −3…3 m in z, and the union radius is √52.25 m

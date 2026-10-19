## ADDED Requirements

### Requirement: Frame
A frame SHALL draw four corner squares of three times the unit and two edge bars one unit thick. Corners SHALL use 0.4, 0.3, 0.4, 0.45. Edges SHALL use 0.475, 0.3, 0.475, 0.35. Side bars SHALL appear only when four times the unit is below the height. State bit 0x10 selects ink 0. A clear mark and a clear wide bit select ink 2. Any other clear hot mask selects ink 0, and a set hot mask selects ink 3.

#### Scenario: Unit 5 places the corners
- **WHEN** unit 5 draws a frame at 10, 20 of size 100 by 20, and again at height 80 with state bit 0x20
- **THEN** the first stores six quads and ink 2, and the second stores eight quads and ink 3

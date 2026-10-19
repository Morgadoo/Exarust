## ADDED Requirements

### Requirement: Skin textured strip
With floats 10, 20, 30 and 40 the strip SHALL draw eight vertices: (30, 20), (10, 20), (30, 30), (10, 30), (30, 30), (10, 30), (10, 40), (30, 40). It SHALL count one begin, four binds in slot order 0x12c, 0x134, 0x138, 0x130, and one end.

#### Scenario: The four bands
- **WHEN** the strip is drawn with 10, 20, 30 and 40
- **THEN** the middle y is 30
- **AND** the eight vertices are those listed, in that order
- **AND** the counts are one begin, four binds and one end

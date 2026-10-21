# Spec Delta

## ADDED Requirements

### Requirement: Door portals
The system SHALL give each door link a portal on the cells' shared edge, spanning the edge's middle ± a half width from −50 cm up to a top: ±82 cm to 245 cm when plane B's side nibble is 0, ±115 cm to 290 cm otherwise, and ±200 cm to 340 cm on a special tile.

#### Scenario: Wide door
- **WHEN** a door link's cell has a non-zero side nibble on that side
- **THEN** its portal is 230 cm wide and runs from 50 cm below the floor to 290 cm above it

### Requirement: Light reach and spill
The system SHALL answer whether light passes between two points as the original does: both in room cells; the same room always; otherwise an open door link of one point's room into the other point's cell whose point ¾ of the way into that cell lies within 2 m of the other point. A light's spill SHALL be the rooms behind its own cell's open door links when it stands within 2 m of that cell's centre. Cells SHALL be indexed as row × width + column, checked only against the cell count.

#### Scenario: Spill from the cell centre
- **WHEN** a light stands 1.7 m from a door's ¾ point but 2.8 m from its cell's centre
- **THEN** it spills into no other room

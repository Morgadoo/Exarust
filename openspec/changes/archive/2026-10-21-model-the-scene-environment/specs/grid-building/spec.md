# Spec Delta

## ADDED Requirements

### Requirement: Rotated special masks
The system SHALL turn a special tile's edge and corner mask by the cell's rotation (each edge nibble and corner triple one side per quarter turn, flag bits kept) before suppressing the covered pieces.

#### Scenario: Rotated trim
- **WHEN** a cell's special tile of type 2 has rotation 1
- **THEN** the mask 7070FFFF becomes 0707FFFF and the other two edges lose their regular pieces

### Requirement: Special piece choice
The system SHALL place the nth special piece of the cell's type; past the last it SHALL place the last, and with no piece of the type the wall set's first special piece.

#### Scenario: Index past the last
- **WHEN** a cell asks for the third piece of a type the set has two of
- **THEN** the second is placed

### Requirement: Terrain floor set
The system SHALL skip, without loading it, only a floor set whose name is exactly `Terrain`.

#### Scenario: Terrain-named set
- **WHEN** a grid names the floor sets `Terrain` and `GEFS_Terrain01`
- **THEN** the first brings no floor pieces and the second is loaded as usual

### Requirement: Node room lookup
The system SHALL sort a node or character into the room of the grid cell it stands in, `floor(x / 6 m) + half the grid`, clamped to the grid for nodes.

#### Scenario: Just below zero
- **WHEN** a node stands 10 cm on the negative side of a cell boundary
- **THEN** it belongs to the cell on that side, not the nearer centre

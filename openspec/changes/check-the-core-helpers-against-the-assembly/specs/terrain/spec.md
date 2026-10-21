# Spec Delta

## ADDED Requirements

### Requirement: Brush table and limit
The palette's rules and then its materials SHALL be inserted into a table sorted by their 16-byte names with A–Z lowered, by binary search (an equal name met by the search stops it there), and looked up by the same search. A tile SHALL take at most eleven painted brushes; at the next brush found it SHALL report "Maximum brushes exceeded." and read no further layers.

#### Scenario: Underscores
- **WHEN** "b", "_a" and "a" are inserted
- **THEN** the table reads "_a", "a", "b"

#### Scenario: Twelve brushes
- **WHEN** a tile paints twelve known brushes
- **THEN** eleven are evaluated and the limit is reported

### Requirement: Terrain height-rule scaling
Height rules (kind 0) SHALL be rescaled as bound × scale + offset in single precision, without rounding; rules of other kinds SHALL keep their bounds.

#### Scenario: A bound
- **WHEN** a height rule's bound 0.25 is scaled by 100 with offset 5
- **THEN** it is 30

### Requirement: Terrain material bytes and merge
A vertex's material weight SHALL be added to its material's byte at row·256 + column as weight × 255 rounded to nearest even, capped at 255. The used-material merge SHALL run over the 241 × 241 corner, all materials but the first and the last, each byte becoming its share of the running weight times 255, rounded to nearest even and capped at 255.

#### Scenario: A merge step
- **WHEN** a mask byte of 100 merges into a running weight of 100
- **THEN** the weight is 200 and the byte 128

## ADDED Requirements

### Requirement: Welding
The system SHALL weld every stream that owns its channel and is not fixed: its elements sorted by their first component with a middle-pivot quicksort, each later element merged into the earliest live one whose squared distance is under 0.0001 cm² (positions) or 1e-9 (texture coordinates), the merged corners spliced before the survivor's head, and the survivors renumbered from zero in their old order, with streams sharing the channel following.

#### Scenario: Positions a hundredth of a centimetre apart
- **WHEN** two positions lie 0.009 cm apart
- **THEN** they weld, and two lying 0.011 cm apart do not

#### Scenario: Equal texture coordinates
- **WHEN** three corners carry texture coordinates (0, 0), (0, 0) and (1, 0)
- **THEN** the second element survives and the corners name elements 0, 0 and 1

### Requirement: Welded skin influences
The character weld SHALL fold a welded-away position's skin influences into its survivor (a bone already there keeps the larger weight, the rest fill the free slots up to four) and SHALL divide every element with two or more influences by their sum.

#### Scenario: Influences fold into the survivor
- **WHEN** a position with influence (bone 1, 0.25) absorbs one with (bone 1, 0.5) and (bone 2, 0.2)
- **THEN** it holds bones 1 and 2 weighted 0.5/0.7 and 0.2/0.7

### Requirement: Winding flip
The system SHALL reverse every face by swapping the first and third element of each corner triple on every channel, leaving the stream values in place.

#### Scenario: One triangle
- **WHEN** the triangle (0, 1, 2) is flipped
- **THEN** every channel holds (2, 1, 0)

### Requirement: UV sphere layout
The system SHALL build the mind effects' UV sphere of `segments` columns and `segments / 2` rows as (rows + 1)(segments − 1) + segments vertices in column pairs of triangle strips: a pair's first strip from column c's north pole between columns c and c + 1 to the south pole, its second strip patched to zigzag between columns c + 2 and c + 1, and the last pair's second strip writing the seam column `segments` itself.

#### Scenario: Four segments
- **WHEN** a sphere of 4 segments is built
- **THEN** it has 13 vertices and the strips (0, 1, 2, 3), (4, 7, 2, 5), (6, 7, 8, 9), (10, 11, 8, 12)

### Requirement: UV sphere vertices
The sphere's columns and rows SHALL both step by 2π/segments, and its directions SHALL come from the engine's fast sine (its quadrant rounded to nearest) and the double-precision cosine, with the z terms multiplied in double.

#### Scenario: The fast sine
- **WHEN** the fast sine of 0.5 is taken
- **THEN** its bits are 0x3EF57746

#### Scenario: A double z term
- **WHEN** a sphere of 8 segments and radius 2 is built
- **THEN** vertex 12's z is exactly −1

## MODIFIED Requirements

### Requirement: Subdivided texture coordinates
The system SHALL subdivide texture coordinates by the same rules inside each coordinate island, and an edge over which the texture folds SHALL become a seam taking the mean opposite coordinate projected onto it. The fold test SHALL be an ordered "greater than zero", so an edge whose two coordinates coincide (a NaN test) SHALL take the plain rule.

#### Scenario: A texture fold
- **WHEN** the second face's third corner is mapped to (0.6, 0), on the same side of the diagonal as the first face's
- **THEN** the diagonal's texture point is (0.475, 0.475)

#### Scenario: A zero-length texture edge
- **WHEN** both ends of the square's diagonal are mapped to (0, 0)
- **THEN** the diagonal's texture point is (0.125, 0.125)

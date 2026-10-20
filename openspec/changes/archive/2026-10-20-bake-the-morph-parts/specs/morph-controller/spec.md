## ADDED Requirements

### Requirement: Morph child gather
The gather SHALL index each child by the two ASCII digits its name begins with as (first − '0')·10 + (second − '0'), zero-fill the gaps, and report the largest index written.

#### Scenario: Sparse children
- **WHEN** children at indices 2 and 0 are gathered
- **THEN** the array is three long with the index-1 slot zero, the largest index is 2, and a name of "07" indexes 7

### Requirement: Morph part bake
The bake SHALL sort the parts ascending by ordinal with a selection starting from a best of 100, then set each delta to the part's position minus the model's position.

#### Scenario: Three parts out of order
- **WHEN** parts with ordinals 2, 0 and 1 are baked over a model at one
- **THEN** the order is 0, 1, 2 and each delta is its position minus one

### Requirement: Morph reference parts
With more than nine parts, the bake SHALL subtract part 5's vertices and deltas from parts 6 through 8.

#### Scenario: Ten parts
- **WHEN** ten parts each hold their index as position and vertex
- **THEN** parts 6, 7 and 8 hold their index minus 5, and parts 5 and 9 are unchanged

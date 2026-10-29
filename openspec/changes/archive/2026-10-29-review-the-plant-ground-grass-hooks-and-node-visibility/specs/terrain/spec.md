## ADDED Requirements

### Requirement: Plant ground normal correction
The plant ground query SHALL take the point relative to the cell corner's right neighbour, test `(−x + z)·0.05 − cross.y·0.0025` (with `cross` the relative point crossed with the corner-to-neighbour edge) against 1, use the far diagonal when the test is above 1 or NaN, and add (far) or subtract (near) `0.0025·dot(other edge, cross)` to the height, summing the dot as `(x + y) + z`.

#### Scenario: Both diagonals on a flat block
- **WHEN** a flat 3 × 3 block of 20 cm cells is queried at (0.5, 0.5) with the point 5 cm above the plane at z = 20 and at z = −20
- **THEN** the tests are 2 and −2 and both corrected heights are 0

### Requirement: Grass hook chunk tests
The grass main draw SHALL draw a chunk only when its block word has `0x80`, the view mask meets the chunk node's mask, the node word has no `0x4` and the chunk has instances; the sun-shadow draw SHALL need block `0x80`, instances and the node mask's `0x80000000`; the omni-shadow draw SHALL need block `0x10000` and `0x20000`, instances and the light's mask meeting the chunk's room mask.

#### Scenario: Sun shadow bit on the mask word
- **WHEN** a visible block's chunk with instances has node mask `0x80000000`
- **THEN** it is drawn into the sun's shadow map, and with mask 0 it is not

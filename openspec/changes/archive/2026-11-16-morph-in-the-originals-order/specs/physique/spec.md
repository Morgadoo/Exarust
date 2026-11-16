## ADDED Requirements

### Requirement: Morph apply order
A morphed mesh SHALL start from its base positions and its base origin and add, target by target in order, each offset times the target's weight in place, skipping a weight below about 1.2e-7 in magnitude and applying a NaN weight.

#### Scenario: Two tiny offsets
- **WHEN** two targets at weight 1 each move a position at 1.0 by 2^-24
- **THEN** the position stays 1.0, as the original's in-place sums give

### Requirement: Transfer neighbour order
The transfer SHALL walk an off-body element's neighbours in the mesh's incidence as its load leaves it (the element's last face first, each face's corners last first, split into smoothing-key runs), each the element at its face's next corner.

#### Scenario: Two faces
- **WHEN** an element off the body has a neighbour on bone 1 in its first face and one on bone 2 in its second
- **THEN** its filled skin record holds bone 2 in its first slot

### Requirement: Morphed box refit
A morphed mesh's box and sphere SHALL be refitted as after any mesh edit: the box of its positions with half extents at least 1e-4 and then 0.001 cm, and the farthest position's squared distance rooted above about 1.2e-7 cm².

#### Scenario: A flat part
- **WHEN** a morphed part lies in one plane
- **THEN** its box keeps 0.001 cm across the plane

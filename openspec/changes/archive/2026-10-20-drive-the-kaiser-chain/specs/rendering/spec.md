## ADDED Requirements

### Requirement: Kaiser chain
The mip chain SHALL generate level L while both destination dimensions stay above zero, their sum is at least 3, and both divide the base exactly. Each level SHALL be resampled from the base with its own Kaiser table, horizontally then vertically.

#### Scenario: An eight-square and an odd size
- **WHEN** an 8×8 uniform image is chained
- **THEN** two levels are produced, 4×4 and 2×2, both uniform
- **WHEN** the image is 3×3 or 1×1
- **THEN** no level is produced

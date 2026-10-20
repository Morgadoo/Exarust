## ADDED Requirements

### Requirement: Morph point blend
Blending SHALL add each source point times the weight to the matching destination point. The result SHALL have the shorter of the two counts.

#### Scenario: Weighted add and mismatch
- **WHEN** the destination is 1, 2, 3, the source 2, 4, 6 and the weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** the source has fewer points
- **THEN** only the matching points are produced

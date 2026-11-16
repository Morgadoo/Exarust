## ADDED Requirements

### Requirement: Box outside a volume
The system SHALL measure how far an oriented box lies outside a set of planes as the largest non-negative clearance, from 0, over the planes its centre is not behind (a NaN plane value goes on): the plane value less the box's support along the normal taken into the box's frame; a NaN clearance SHALL never win. The terrain's hidden-block light rank SHALL use it.

#### Scenario: Two faces
- **WHEN** a box of half side 0.5 sits 2 units along +X from a unit box's centre
- **THEN** it lies 0.5 outside

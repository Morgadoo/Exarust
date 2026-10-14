## ADDED Requirements

### Requirement: Shadow casters
The system SHALL cast shadows as the original does. Alpha-tested surfaces are cut in the shadow maps the way their colour is cut. Surfaces with opacity ≤ 0.8, or flagged by the material record, cast no shadows.

#### Scenario: Cobwebs in torchlight
- **WHEN** a torch lights a wall hung with cobwebs
- **THEN** the cobwebs' shadows follow their strands instead of darkening the wall in solid sheets

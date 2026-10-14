## ADDED Requirements

### Requirement: Terrain texture detail
The system SHALL read terrain colour and normal layers at their base level, as the original's mip-less composites show them.

#### Scenario: Floor seen from the game's camera
- **WHEN** a cobbled terrain floor is seen from the game's camera height
- **THEN** it shows the stones' full grain, not a mip-blurred surface

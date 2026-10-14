## ADDED Requirements

### Requirement: Terrain composite
The system SHALL shade terrain from the original's composite rules. Gloss comes from the layers' gloss textures (roughness 1 − gloss), the albedo is × 0.75, and parallax comes from the bump slot's height at 0.007 per 6 m block.

#### Scenario: Dry dungeon floor
- **WHEN** a torch lights a terrain floor of cobblestones
- **THEN** the stones look matte, with soft relief, not wet

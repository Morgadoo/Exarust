# Spec Delta

## ADDED Requirements

### Requirement: Height pass
The system SHALL displace terrain vertices by the palette materials' height maps, layered with the tile's opacities and blend masks, as the original's height pass does.

#### Scenario: Neutral ground over a dungeon floor
- **WHEN** neutral terrain is painted over a dungeon floor at the same height
- **THEN** the terrain rests just above the floor instead of z-fighting with it

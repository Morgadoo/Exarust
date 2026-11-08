## ADDED Requirements

### Requirement: Sector-major blended walk
In a level with room sectors the transparent phase SHALL draw the decal list sector by sector in scene order (the scene's own sector, then the rooms in room order), then the blended list the same way, each sector's surfaces in state rank order, and then everything without a state by view depth.

#### Scenario: Glass in two rooms
- **WHEN** a glass of the first blended rank stands in the second room and a glass of the second rank in the first room
- **THEN** the first room's glass is drawn first

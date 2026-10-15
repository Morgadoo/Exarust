## ADDED Requirements

### Requirement: Terrain in the GI volume
The system SHALL fill the GI occupancy below the terrain surface, as the original does, so light does not drain beneath terrain floors.

#### Scenario: Room on terrain
- **WHEN** a torch burns in a room whose floor is terrain
- **THEN** its bounce light stays in the room and reaches surfaces in shadow

### Requirement: GI light selection
The system SHALL inject into the GI the lights whose reach overlaps the current block, nearest first.

#### Scenario: Large level
- **WHEN** the camera is in a part of a level with hundreds of lights
- **THEN** the lights near it are the ones lighting the GI volume

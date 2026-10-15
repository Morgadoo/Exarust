## ADDED Requirements

### Requirement: Lights per mesh
The system SHALL light each mesh that takes part in light linking only with its six strongest links, strongest by link weight, as the original lights an object. A linked mesh with no links SHALL receive no point light. Terrain tiles and meshes that cast no shadows SHALL be lit by every light in reach.

#### Scenario: Many lights in reach
- **WHEN** eight lights are linked to a mesh
- **THEN** only the six with the largest link weights light it

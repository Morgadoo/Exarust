## ADDED Requirements

### Requirement: Interface camera sees no world mesh
The loading screen's camera, which stays to draw the interface over the game's view, SHALL belong to no render layer, so that it lists no world mesh in its view and marks none as seen; the interface SHALL still draw through it.

#### Scenario: Terrain inside its box
- **WHEN** a level's terrain blocks lie inside the interface camera's orthographic box but outside the game camera's view
- **THEN** the interface camera lists none of them and they count as not in view

## ADDED Requirements

### Requirement: Terrain shading
The system SHALL shade terrain with the original's lighting: its lights and shadows, the sun, the GI or environment ambient and reflections, Fresnel F0 0.04, and a gloss taken from the ground's colour.

#### Scenario: Light on a terrain floor
- **WHEN** a level light hangs above a terrain floor
- **THEN** the floor shows the light's pool, lit in the same units as the models around it

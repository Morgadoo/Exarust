## ADDED Requirements

### Requirement: Shadow casters per light
The system SHALL let a mesh cast into a light's shadow map only when the two are linked as the original links them: their bounding spheres overlap, (1 − 0.8·d/R)⁴ · luma(colour · intensity) > 0.01 with d the distance from the light to the mesh's sphere, and the mesh is not in the light's own object. Links SHALL follow moving lights and meshes.

#### Scenario: Distant mesh
- **WHEN** a mesh lies within a light's range but where its weight is below 0.01
- **THEN** the mesh casts no shadow from that light

#### Scenario: Torch handle
- **WHEN** a torch's light renders its shadow map
- **THEN** the torch's own handle is left out of it, but other lights still get the handle's shadow

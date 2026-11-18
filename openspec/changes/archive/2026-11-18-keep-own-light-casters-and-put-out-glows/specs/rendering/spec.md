## ADDED Requirements

### Requirement: Own-object casters keep the keep-own-light meshes
A light's cube shadow map SHALL leave out the meshes of its parent's children, its parent and its grandparent, except a mesh whose render word carries the keep-own-light bit `0x1000`, which casts into its own light.

#### Scenario: A flagged mesh beside the light
- **WHEN** a light's parent has a mesh child with the keep-own-light bit and one without it
- **THEN** only the mesh without the bit is left out of the light's shadow map

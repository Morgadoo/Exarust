## ADDED Requirements

### Requirement: Grass program
Terrain plants SHALL be shaded with the original's grass program: roughness `clamp(1 − 3(g − 0.8r), 0.01, 0.99)²` of the diffuse map's colour, each light's diffuse as plain N·L with no Fresnel share and its highlight with Fresnel 0.04 and no light-side geometry term, the normal never flipped on a back face, and three quarters of the specular sum added. The normal map SHALL be used only above shading quality 3, and the plants SHALL not sway.

#### Scenario: Back of a leaf
- **WHEN** the sun lights a plant whose back faces the camera
- **THEN** the leaf is lit with its own normal, not with a normal flipped toward the camera

#### Scenario: Low shading quality
- **WHEN** the shading quality is 3
- **THEN** the plants draw without their normal map

## ADDED Requirements

### Requirement: Texture sampling
The system SHALL sample material textures as the original does at its top texture quality with 4× supersampling. Filtering is 16× anisotropic. Colour textures get a −0.75 LOD bias; normal, gloss and packed specular maps get none.

#### Scenario: Floor at a grazing angle
- **WHEN** a floor is seen at a grazing angle
- **THEN** its texture keeps fine detail instead of blurring to a low mip level

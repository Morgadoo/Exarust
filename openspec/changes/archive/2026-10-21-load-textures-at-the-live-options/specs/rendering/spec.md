# Spec Delta

## MODIFIED Requirements

### Requirement: Texture sampling
The system SHALL sample material textures as the original does at the live texture quality, texture detail and super-sampling options, which at the retail user's settings (quality 5, detail 3, 4× super-sampling) give 16× anisotropic filtering, a −0.75 LOD bias for colour textures and none for normal, gloss and packed specular maps. A change of those options SHALL apply to textures decoded afterwards.

#### Scenario: Floor at a grazing angle
- **WHEN** a floor is seen at a grazing angle
- **THEN** its texture keeps fine detail instead of blurring to a low mip level

#### Scenario: Lower texture quality
- **WHEN** the live texture quality is 4 and the detail 2
- **THEN** textures decoded afterwards take the quality's mask, anisotropy level and −0.5 bias base, and detail 2

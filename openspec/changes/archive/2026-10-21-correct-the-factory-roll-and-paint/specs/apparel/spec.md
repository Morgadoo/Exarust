## ADDED Requirements

### Requirement: Procedural material programs
The system SHALL paint each procedural material with the original's programs for its family: material-scale textures repeating by the material's first colour texture, damage-scale ones once per 1024 texels, each layer's colour and wear weight, and the specular pass measuring its creases on the painted normal map after every layer's normal pass. Plate materials SHALL read their own rust grain map.

#### Scenario: Pale linen
- **WHEN** rough cloth in palette colour 1 at quality 160 and damage 64 is painted over a mid-grey shade map and mid-grey library textures
- **THEN** its colour texel is 96

#### Scenario: Plate grain
- **WHEN** material 13 is painted
- **THEN** its colour, specular and normal passes read `MtlRustA04G`

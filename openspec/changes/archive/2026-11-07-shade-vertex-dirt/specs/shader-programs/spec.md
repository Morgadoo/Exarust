## ADDED Requirements

### Requirement: Vertex dirt mode choice
A record SHALL show dirt mode 1 (grime) or 2 (wear) only when its factory shader word selects the PBR program and that mode (diffuse feature bit, a dirt layer, extra flags `0x7000` of 0 or `0x1000`) and its dirt map loads as a colour texture. Mode 3, the other programs and meshes without a colour stream SHALL show no dirt.

#### Scenario: Hair has none
- **WHEN** a hair record carries a dirt layer and extra flags 0
- **THEN** it shows no grime

### Requirement: Grime shading
Grime SHALL pull the dirt texel d toward ½ by min(α·(2 − amount), 1) for vertex alpha α, multiply the diffuse colour by 2·d and make the roughness r into 1 − (1 − r)(1 − 2|½ − d.g|).

#### Scenario: Clean grime
- **WHEN** a grime surface with dirt amount 0.5 draws a vertex of alpha 255
- **THEN** its colour and roughness are those without dirt

### Requirement: Wear shading
Wear SHALL read d at the texture coordinates times the diffuse map's aspect ((w/h, 1) or (1, h/w)) and, with w = clamp(α·3|0.4 − d.r|, 0, 1), make the roughness 1 − (1 − r)·w, F0 mix(0.04, F0, w) and the diffuse colour c mix(0.75·α·d, c, α)·α.

#### Scenario: Worn to black
- **WHEN** a wear surface draws a vertex of alpha 0
- **THEN** its diffuse colour is black, its roughness 1 and its F0 0.04

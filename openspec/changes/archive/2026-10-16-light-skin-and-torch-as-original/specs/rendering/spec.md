## ADDED Requirements

### Requirement: Built-in skin
The system SHALL draw the skin of human and ancient bodies as the original's built-in skins instead of their `skinhuman` record, with the race's own textures named after the body model's stem (`<stem>difm`, `<stem>nrmm0`, `<stem>spcm`). The diffuse colour SHALL be the colour texture's stored texels times the middle skin tone (0.46, 0.47, 0.47), decoded from sRGB after the product.

#### Scenario: Lit skin in the character view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the chest facing the key light shows a mid tan near sRGB (100, 81, 67), neither clipped to white nor blooming

#### Scenario: Torchlit skin
- **WHEN** `exarust play exanima01` runs
- **THEN** the player's skin near the carried torch is warm-lit and does not blow out to white

#### Scenario: Ancient body
- **WHEN** `exarust --race 2 character` runs
- **THEN** the ancient's body shows its own desiccated skin texture, not an untextured glossy figure

### Requirement: Skin lighting
The system SHALL light built-in skin with F0 0.04 and roughness 1 − 0.8 × the gloss texture, plus a subsurface share of the diffuse colour, sRGB (1.0, 0.8, 0.46) for humans and (1.0, 0.63, 0.133) for ancients, lit by a wrapped term on the normal map at mip 2.

#### Scenario: Soft terminator
- **WHEN** a body is lit from one side in the character view
- **THEN** the skin turns warm and soft where light meets shadow instead of ending in a hard edge

## MODIFIED Requirements

### Requirement: Built-in skin
The system SHALL draw the skin of human and ancient bodies as the original's built-in skins instead of their `skinhuman` record, with the race's own textures named after the body model's stem (`<stem>difm`, `<stem>nrmm0`, `<stem>spcm`, or the women's `diff`, `nrmf0` and `spcf` for a female record). The diffuse colour SHALL be the colour texture's stored texels times the character's skin tone multiplier, decoded from sRGB after the product.

#### Scenario: Lit skin in the character view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the chest facing the key light shows a mid tan near sRGB (100, 81, 67), neither clipped to white nor blooming

#### Scenario: Torchlit skin
- **WHEN** `exarust play exanima01` runs
- **THEN** the player's skin near the carried torch is warm-lit and does not blow out to white

#### Scenario: Ancient body
- **WHEN** `exarust --race 2 character` runs
- **THEN** the ancient's body shows its own desiccated skin texture, not an untextured glossy figure

## ADDED Requirements

### Requirement: Skin tone
The system SHALL compute a character's skin tone multiplier as the original does from the record's two tone words: its HSV tone table decoded to linear colours, interpolated within the grid half each word falls in, divided by the base texture's colour, scaled by 0.6, encoded and scaled by 1.4. A body without a record SHALL take the middle tone.

#### Scenario: Tone words
- **WHEN** a record's tone words are both 0x8000
- **THEN** the multiplier is about (0.461, 0.469, 0.467)
- **WHEN** they are 0 and 0xFFFF
- **THEN** the multiplier is about (0.293, 0.295, 0.287), the darkest tone

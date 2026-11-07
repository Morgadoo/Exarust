## ADDED Requirements

### Requirement: Float filter layouts
The resample to an arbitrary size and the image sharpen SHALL take only the six layouts the unit converts (the format word masked by `0x800CFFFF` equal to `0x2008`, `0x4200`, `0x6208`, `0x8400`, `0xC600` or `0xE608`): the resample's windowed path SHALL return no image and the sharpen SHALL leave the image unchanged for any other word, while a power-of-two resample SHALL still take the chain's level.

#### Scenario: A float image
- **WHEN** an RGBAF image is sharpened
- **THEN** it is returned unchanged

### Requirement: One-texel sharpen axes
The image sharpen SHALL copy an axis one texel long instead of filtering it.

#### Scenario: A single row
- **WHEN** a 4 × 1 image is sharpened
- **THEN** only its row is filtered and the result equals the 1 × 4 image's column filtered with the same kernel

### Requirement: Icon resize on add
An icon atlas SHALL add an icon of its cell's square as it is, and SHALL resize any other icon to the cell with the resample to an arbitrary size after clearing its flags `0x70000`, then sharpen it by amount 0.1 and radius 1 when the atlas sharpens.

#### Scenario: The HUD status icon at four pixels a unit
- **WHEN** the 96-pixel skill cell enters the 32-pixel status atlas
- **THEN** it is resampled to 32 pixels and sharpened

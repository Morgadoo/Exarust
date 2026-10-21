## ADDED Requirements

### Requirement: Texture creation word
A material texture's creation word SHALL be the material record's texture load flags or-ed with the texture quality mask, plus the data and no-bias bits for normal, gloss and height maps and for the specular map of a material whose kind has bit 0, and the no-bias bit for a diffuse map whose subsurface red, green and blue have a squared length above zero. The image's header flags from bit 16 up SHALL join the word.

#### Scenario: The file's wrap bit
- **WHEN** a request with word 0xF0 loads an image whose header flags are 0x20010000
- **THEN** the texture repeats across its width and clamps along its height

### Requirement: Texture sampler
The sampler SHALL follow the creation word: bit 5 linear magnification, bit 4 linear minification, bit 6 mipmapping and bit 7 linear blending between levels, bits 16 and 17 repeat (else clamp). Anisotropy SHALL be 2^(level − mode) for a mode (bits 0–1) below the quality's level and not 3, else 1, and 1 unless every filter is linear. The LOD bias SHALL be the base times (the supersampling factor times the single-precision third).

#### Scenario: A material texture at the retail settings
- **WHEN** a material with load flags 0x70000 asks for a diffuse map at texture quality 5
- **THEN** its word is 0x700F0, it repeats on every axis, filters trilinearly with 16× anisotropy and has an LOD bias of −0.75 at 4× supersampling

#### Scenario: Anisotropy modes
- **WHEN** the quality level is 4 and the word's mode is 1, 2 or 3
- **THEN** the anisotropy is 8, 4 and 1

#### Scenario: The quality table
- **WHEN** texture quality 0, 3, 4 and 5 are set
- **THEN** the mask, anisotropy level and bias base are 0x00, 0, 0; 0x70, 1, 0; 0xF0, 2, −0.5; and 0xF0, 4, −0.75

### Requirement: Colour space of an upload
A texture SHALL be uploaded as sRGB only when the request asks for colour, its word lacks the data bit and its stored format is colour. An uncompressed format word with bit 16 set, and a compressed one whose top byte is 0x82 or 0x92, SHALL count as data.

#### Scenario: Data words
- **WHEN** images with the words 0x0111C600, 0x1111C600, 0x823BC600 and 0x927B8400 are requested as colour
- **THEN** each is uploaded linear
- **AND** 0x813BC600, 0x0100C600 and 0x01004200 are uploaded as sRGB

### Requirement: First uploaded level
The first uploaded level SHALL be the image's detail class (header flags bits 22–23) less `TextureDetail`, plus one, floored at zero, then raised while the width and then the height shifted by it exceed the device's largest texture side. A stored chain SHALL be uploaded from that level, all of it when the quality mask lets the word's mip bit through and that level alone otherwise. This SHALL hold for every image in the retail packs.

#### Scenario: The detail setting
- **WHEN** a 4×4 image of detail class 3 with a stored chain is loaded at `TextureDetail = 3`
- **THEN** its 2×2 level is the uploaded base, followed by the 1×1 level

#### Scenario: Retail images
- **WHEN** every shipped image is converted at the retail settings
- **THEN** none fails, each image's stored-chain flag agrees with the levels its data holds, and the 21 images of class 3 start at their second level

### Requirement: Single-level images
An uncompressed single-level image SHALL get its generated chain from the first uploaded level. A compressed image without a stored chain SHALL keep its base level alone and lose its mip filter.

#### Scenario: A compressed image without a chain
- **WHEN** an 8×8 BC1 image of class 3 stores only its base level
- **THEN** it is uploaded at 8×8 with one level and no mip filter, on the GPU and the CPU path alike


### Requirement: Failure placeholder
An image that is shipped but cannot be read SHALL be uploaded as the original's placeholder: a 1×1 RGB8 image of (64, 64, 64), opaque, repeating on every axis, in the colour space the request asks for. An image the game does not ship SHALL leave the slot without a map.

#### Scenario: An unreadable image
- **WHEN** a material's diffuse image cannot be parsed
- **THEN** the slot gets the 1×1 grey placeholder in sRGB, repeating

### Requirement: Texture manager
Textures SHALL be kept by their stored name: the first sixteen bytes, cut at the last dot among them unless it leads, with A–Z folded to lower case, ordered by the four little-endian 32-bit words compared in turn. Images SHALL be looked up by the stored name. A second request for a live name SHALL take a reference and keep the record and its first creation word; the last release SHALL free it.

#### Scenario: Stored names
- **WHEN** the names `wall01n.rfi`, `a.b.rfi` and `.hidden` are stored
- **THEN** they become `wall01n`, `a.b` and `.hidden`

#### Scenario: Order and first word
- **WHEN** `stone01d`, `brick02d` and `wood03d` are added and `stone01d` is requested again with another word
- **THEN** the records are in the order brick02d, wood03d, stone01d, and stone01d keeps its first word with two references

## MODIFIED Requirements

### Requirement: Stored mip levels
The system SHALL decode and upload every mip level an `.rfi` stores from the first uploaded level on, including for textures decoded on the CPU, so textures keep their resolution up close and filter smoothly at a distance.

#### Scenario: Height map in the distance
- **WHEN** a BC4 height map with a stored chain is decoded on the CPU
- **THEN** its texture carries every stored mip level from the first uploaded one, not only that level

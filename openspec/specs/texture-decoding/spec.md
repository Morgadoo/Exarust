# texture-decoding Specification

## Purpose

Decodes the original `.rfi` images, including every pixel format and storage
mode shipped with the game, so textures can be displayed and uploaded to the GPU.

## Requirements

### Requirement: Image header
The system SHALL read an image's width, height, pixel format, storage mode, mip level count and unpacked data size.

#### Scenario: Full mip chain
- **WHEN** a 1024x1024 BC1 image stores 699 064 bytes of pixel data
- **THEN** it reports 11 mip levels

#### Scenario: Base level only
- **WHEN** a 1024x1024 BC1 image stores 524 288 bytes of pixel data
- **THEN** it reports 1 mip level

### Requirement: Pixel formats
The system SHALL decode BC1, BC3, BC4 (luminance), BC5 (two-channel normal) and uncompressed RGB8, RGBA8, L8, LA8 and A8 images to RGBA8. Uncompressed colour images keep their stored channel order, red first, in raw and packed storage alike, as the original uploads them; this holds for every RGB8 and RGBA8 image in the retail packs.

#### Scenario: Normal map
- **WHEN** a BC5 normal map is decoded
- **THEN** red and green hold the stored X and Y, blue holds the reconstructed Z, and alpha is opaque

#### Scenario: Uncompressed colour order
- **WHEN** an uncompressed colour image is decoded, raw or packed
- **THEN** its stored first, second and third bytes are output as red, green and blue, and a stored fourth byte as alpha

#### Scenario: RGB8 normal map
- **WHEN** an uncompressed RGB8 normal map of the retail packs is decoded
- **THEN** blue holds its Z, about 255 on flat areas, and red and green its X and Y

#### Scenario: Embers' glow
- **WHEN** the packed RGB8 glow map `embersael` is decoded
- **THEN** it is orange: red brightest, then green, then blue

#### Scenario: Unknown format word
- **WHEN** an image declares a format word outside the known set
- **THEN** decoding fails with an error naming the word

### Requirement: Packed storage
The system SHALL unpack run-length-encoded images to exactly the declared data size before decoding.

#### Scenario: Font atlas
- **WHEN** the packed `fontbase24r.rfi` is decoded
- **THEN** a 1024x1024 alpha atlas of glyphs is produced

#### Scenario: Corrupt stream
- **WHEN** a packed stream ends before the declared size is reached
- **THEN** decoding fails with a truncation error

### Requirement: Retail coverage
The system SHALL decode the base level of every image in the retail packs.

#### Scenario: Corpus decode
- **WHEN** every image in Textures, Apparel, ComponentTex, Emblems and Resource is decoded
- **THEN** none fails

### Requirement: Stored mip levels
The system SHALL decode and upload every mip level an `.rfi` stores from the first uploaded level on, including for textures decoded on the CPU, so textures keep their resolution up close and filter smoothly at a distance.

#### Scenario: Height map in the distance
- **WHEN** a BC4 height map with a stored chain is decoded on the CPU
- **THEN** its texture carries every stored mip level from the first uploaded one, not only that level

### Requirement: Pixel-format conversion
The system SHALL know the original's 20 pixel formats (RGB8, LUM8, RGBA8, LUMA8, ALPHA8, LUM1, XYZ8, XY8, VALA8, XYZV8, VAL8, AB8, ABC8, ABCD8, RGBf, RGBAf, Lf, Af, XYZf, Vf) with their ids, fallbacks and pixel sizes, and SHALL convert between them as the original's converters do, declining the targets they decline and returning errors, never panicking, on unknown sources or short buffers. Uncompressed images SHALL be decoded through the stored format's converter.

#### Scenario: Luma
- **WHEN** the RGB pixel (10, 200, 30) is converted to LUM8
- **THEN** it becomes 124
- **AND** to LUMA8 it becomes (124, 1)

#### Scenario: Rebuilt Z
- **WHEN** the two-channel pixel (200, 128) is converted to XYZ8
- **THEN** it becomes (200, 128, 232)

#### Scenario: Floats to bytes
- **WHEN** the floats 0.5, −0.5 and 2.0 are converted from RGBf to RGB8
- **THEN** they become 128, 0 and 255
- **WHEN** the Vf value 1.5 is converted to RGB8
- **THEN** each channel is 126, the low byte of 382

#### Scenario: A declined target
- **WHEN** a LUMA8 image is converted to LUM8
- **THEN** the converter declines and nothing is written

### Requirement: Texture creation word
A material texture's creation word SHALL be the material record's texture load flags or-ed with the texture quality mask, plus the data and no-bias bits for normal, gloss and height maps and for the specular map of a material whose kind has bit 0, and the no-bias bit for a diffuse map whose subsurface red, green and blue have a squared length above zero. The image's header flags from bit 16 up SHALL join the word.

#### Scenario: The file's wrap bit
- **WHEN** a request with word 0xF0 loads an image whose header flags are 0x20010000
- **THEN** the texture repeats across its width and clamps along its height

### Requirement: Texture sampler
The sampler SHALL follow the creation word: bit 5 linear magnification, bit 4 linear minification, bit 6 mipmapping and bit 7 linear blending between levels, bits 16 and 17 repeat (else clamp). Anisotropy SHALL be 2^(level − mode) for a mode (bits 0–1) below the quality's level and not 3, else 1, and 1 unless every filter is linear.

#### Scenario: A material texture at the retail settings
- **WHEN** a material with load flags 0x70000 asks for a diffuse map at texture quality 5
- **THEN** its word is 0x700F0, it repeats on every axis, filters trilinearly with 16× anisotropy and has an LOD bias of −0.75 at 4× supersampling

#### Scenario: Anisotropy modes
- **WHEN** the quality level is 4 and the word's mode is 1, 2 or 3
- **THEN** the anisotropy is 8, 4 and 1

#### Scenario: The quality table
- **WHEN** texture quality 0, 3, 4 and 5 are set
- **THEN** the mask, anisotropy level and bias base are 0x00, 0, 0; 0x70, 1, 0; 0xF0, 2, −0.5; and 0xF0, 4, −0.75

### Requirement: Texture LOD bias
The LOD bias SHALL be the base times (the supersampling factor times the single-precision third), evaluated in that order, where the factor is the supersampling level below 2 and that level less one from 2 up. Every material path, painted, race-skin and run-time apparel included, SHALL take the bias.

#### Scenario: Supersampling factor
- **WHEN** the supersampling level is 1, 2 and 4 at a bias base of −0.75
- **THEN** the biases are −0.25, −0.25 and −0.75

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

### Requirement: Single-channel diffuse
A diffuse map stored as BC4, the original's luminance format, SHALL be uploaded as one channel and shown as grey on every material path, with no diffuse bias for a record that has a secondary colour.

#### Scenario: A BC4 diffuse
- **WHEN** a run-time apparel or race-skin material takes a BC4 diffuse
- **THEN** the shader repeats the red channel as grey

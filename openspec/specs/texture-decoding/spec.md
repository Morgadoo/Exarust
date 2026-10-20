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
The system SHALL decode and upload every mip level an `.rfi` stores, including for textures decoded on the CPU, so all textures keep their full resolution up close and filter smoothly at a distance.

#### Scenario: Height map in the distance
- **WHEN** a BC4 height map is decoded on the CPU
- **THEN** its texture carries every stored mip level, not only the base level

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

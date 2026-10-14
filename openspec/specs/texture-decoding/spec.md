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
The system SHALL decode BC1, BC3, BC4 (luminance), BC5 (two-channel normal) and uncompressed BGR8, BGRA8, L8, LA8 and A8 images to RGBA8.

#### Scenario: Normal map
- **WHEN** a BC5 normal map is decoded
- **THEN** red and green hold the stored X and Y, blue holds the reconstructed Z, and alpha is opaque

#### Scenario: Uncompressed colour order
- **WHEN** an uncompressed colour image is decoded
- **THEN** stored blue, green and red bytes are output as red, green and blue

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

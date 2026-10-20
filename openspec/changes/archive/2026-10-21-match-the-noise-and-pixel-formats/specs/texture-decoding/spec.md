## ADDED Requirements

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

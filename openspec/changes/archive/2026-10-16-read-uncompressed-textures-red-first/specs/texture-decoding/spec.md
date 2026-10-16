## MODIFIED Requirements

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

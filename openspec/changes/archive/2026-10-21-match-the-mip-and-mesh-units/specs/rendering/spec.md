## MODIFIED Requirements

### Requirement: Kaiser kernel
The Kaiser table of a level SHALL use n = 8·2^(level−1) taps, h = n/2, c = 2^−level, and w = I0(4√(1−t²))/I0(4)·sinc(u·π·c) where 1 − t² ≥ 0, zero elsewhere. The half width and tap position SHALL be f32, the window and sinc f64, each weight stored as f32, and the weights SHALL be divided by their f64 sum.

#### Scenario: Level one
- **WHEN** the level-1 table is built
- **THEN** it has eight taps, is symmetric, sums to 1 and has larger weights near its centre

#### Scenario: A NaN argument
- **WHEN** the Bessel series is evaluated at NaN
- **THEN** it returns NaN instead of looping

### Requirement: Mip axis pass
The axis pass SHALL use the integer ratio src/dst, start at −((n−ratio)/2), accumulate source × weight per channel in tap order, and wrap each tap modulo the source extent or clamp it to the ends.

#### Scenario: Average, clamp and wrap
- **WHEN** two RGBA texels are averaged by half weights
- **THEN** the output is their mean
- **WHEN** four taps start before the first texel
- **THEN** the clamped pass repeats the first texel and the wrapping pass takes the last

### Requirement: Kaiser chain
The mip chain SHALL generate levels from its first requested level to its last, each side shifted right by the level with a floor of one, and SHALL stop after the level whose two sides sum to two. Each level SHALL be resampled from the base with its own table, horizontally then vertically, wrapping or clamping each axis by the image's own flags (0x10000 across, 0x20000 along the height).

#### Scenario: An eight-square and an odd size
- **WHEN** an 8×8 uniform image is chained
- **THEN** three levels are produced, 4×4, 2×2 and 1×1, all uniform
- **WHEN** the image is 3×3
- **THEN** one 1×1 level is produced

#### Scenario: Clamped edges
- **WHEN** an image without wrap flags has a bright last column
- **THEN** its first level's first texel takes nothing from that column

### Requirement: Kaiser texture mips
An uncompressed texture with one stored level SHALL get the generated chain from its first uploaded level to its end when its word and the quality mask ask for mipmapping, or that level alone otherwise, and its mip level count SHALL be the levels it then holds; uploaded sides SHALL be at least one texel. A compressed texture with one stored level SHALL get no chain, also when it is decoded on the CPU. Every other texture SHALL keep its stored levels.

#### Scenario: One-level and stored-level textures
- **WHEN** a 4×4 RGBA8 texture stores only its base level
- **THEN** a 2×2 and a 1×1 level are appended and the count becomes 3
- **WHEN** a texture stores two levels, or is a single-level BC4 image decoded on the CPU
- **THEN** no level is generated

## ADDED Requirements

### Requirement: Mip bank by format word
A generated chain SHALL filter with the uniform bank (level L: 2^L taps of 2^−L) when the format word has bit 16 set and bit 31 clear, and with the Kaiser bank otherwise.

#### Scenario: A data step
- **WHEN** a data image (ABCD8) with rows 0, 0, 255, 255 is halved
- **THEN** its level reads exactly 0 and 255
- **WHEN** the same rows are halved as a colour image
- **THEN** the step blurs into both outputs

### Requirement: Normal-map mip renormalisation
Each filtered level of a three- or four-channel image whose header flag 0x4000000 is set SHALL have its texels' first three channels normalised before conversion back to bytes, with the squares summed x, z then y, the reciprocal square root estimate, and a zero vector left zero; one- and two-channel images SHALL not be renormalised.

#### Scenario: A signed vector
- **WHEN** a flagged XYZV8 image of signed (1, 1, −1) texels is chained
- **THEN** its first level's texels are bytes 201, 201, 54

#### Scenario: A zero vector
- **WHEN** a zero vector is renormalised
- **THEN** it stays zero

### Requirement: Resample to size
The resample to an arbitrary size SHALL return level k of the image's chain when the destination shifted left by k (1 to 13) equals the source on both axes. Otherwise it SHALL filter both axes with the phase-shifted window, in the space chosen by the word and the sizes: signed words centred, data words and magnified images linear, other words squared, with RGBA8 and LUMA8 colour words squared whatever the sizes.

#### Scenario: A power-of-two ratio
- **WHEN** an 8×8 image is resampled to 2×2
- **THEN** the result equals level 2 of its chain

#### Scenario: A magnified RGB8 image
- **WHEN** a colour RGB8 image is resampled to a larger size
- **THEN** it is filtered linearly

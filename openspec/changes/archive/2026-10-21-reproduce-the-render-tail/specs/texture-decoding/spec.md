## ADDED Requirements

### Requirement: Image size
The system SHALL size one level from the format word's fields (block compressed: whole blocks of 1 + bits 19–23 bytes and 1 + bits 16–18 texels a side; bit-packed: ceil(w·h·bits / 8); otherwise w·h·ceil(bits / 8), the bits being bits 6–12 plus bits 0–5), and one face as its base level plus, only when the header flags carry the chain bit, every halved level while width + height exceeds 2. A face's size SHALL NOT include a cube's six faces.

#### Scenario: A chained BC1 image
- **WHEN** a 1024 × 1024 BC1 image is sized with and without the chain flag
- **THEN** it takes 699 064 and 524 288 bytes

#### Scenario: A cube face
- **WHEN** a 2 × 2 RGB8 image with the cube flag is sized
- **THEN** it takes 12 bytes

### Requirement: Cube images
An image whose header flags under 0x70000000 read 0x40000000 SHALL be a cube of six faces stored back to back, each `data size` bytes apart with its own chain; a packed stream SHALL unpack to `data size` bytes. Every shipped environment cube SHALL decode to six faces.

#### Scenario: The indoor environment cube
- **WHEN** `genref01r` is read from the installation
- **THEN** it is a cube of six 256² faces with nine levels each and a data size of 262 143

### Requirement: Image copies
A full copy SHALL hold `data size` bytes of each face, unpacking a packed source first; a base-level copy SHALL clear the chain flag, size the base level and take each face's base from the source one source `data size` apart. A created image SHALL be sized from its flags (the chain included when they ask) with six faces for a cube, and a render target without a detail class SHALL get class 2.

#### Scenario: A two-level cube's base copy
- **WHEN** a cube of 2 × 2 RGB8 faces with a chain (15 bytes a face) is copied without its chain
- **THEN** the copy's data size is 12 and its second face is the source's bytes 15–26

### Requirement: Image conversion
Converting an image SHALL do nothing for its own word; SHALL only replace the word for a requested word with bit 20 set and bit 31 clear and the same layout under 0x8F0CFFFF; and otherwise SHALL convert every pixel of every face and stored level with the converter of the first registered format whose id is the image's word, a zero request taking that format's fallback, keeping the chain flag. An unregistered word or a declined target SHALL leave the image unchanged.

#### Scenario: Grey to its fallback
- **WHEN** a 2 × 1 grey image with values 7 and 9 is converted with a zero request
- **THEN** it is RGB8 with pixels (7, 7, 7) and (9, 9, 9)

#### Scenario: A chained image
- **WHEN** a 2 × 2 RGB8 image with its 1 × 1 level is converted to RGBA8
- **THEN** its data size is 20 and the chain flag stays

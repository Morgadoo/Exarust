# Spec Delta

## ADDED Requirements

### Requirement: Render list choice
A material's flag word SHALL choose its list in this order: bit `0x10000` the decal list, then bit `0x8000` the alpha-tested list, then the low nibble (0 opaque, 1 to 4 blended, anything else no state). The second renderer mode SHALL send every class but a blending nibble to the decal list and file a blending nibble in a list of its own.

#### Scenario: The decal bit wins
- **WHEN** a flag word has bits `0x10000` and `0x8000`
- **THEN** the decal list is chosen

### Requirement: Existing state lookup and released flags
The lookup of an existing state SHALL ignore bit `0x10000` and the renderer mode, and a released state SHALL keep the word with bits 21 and 23 cleared for detail levels below 3 and 4.

#### Scenario: The decal bit in a lookup
- **WHEN** an existing state is looked up for the word `0x10000`
- **THEN** the opaque list is found, as the low nibble says

### Requirement: Deferred draw queue
The queue SHALL hold at most 1023 entries, each in the bucket of its sector's light count, counting the enabled lights whose weight, widened to double, is not below 0.004, capped at the renderer's count and six (a NaN weight counts). Appending to a full queue SHALL flush it, and a flush SHALL remember its last entry's bucket.

#### Scenario: A full queue
- **WHEN** a draw is appended to a queue holding 1023 entries
- **THEN** the queue flushes first and then holds that draw alone

### Requirement: Draw queue order
A draw whose bucket is the one the lights were last set up for SHALL go to the head once; any other SHALL go right behind the first entry of its bucket that has a successor, or at the end, so a group reads first entry then newest first.

#### Scenario: A repeated bucket
- **WHEN** three draws of one bucket are appended to an empty queue not set up for it
- **THEN** a flush draws the first, then the third, then the second

### Requirement: Still capture plan
A still capture SHALL take eight samples, each blended in at `1 - 1/(index + 1)` (the first replaces the frame), with the projection offset `2 * scale * jitter / size` per axis from the recorded jitter table starting at entry 10, then run the post chain and downsample to half size (`width >> 1`, `height >> 1`) as RGBA8.

#### Scenario: Blend weights
- **WHEN** the eight blend weights are listed
- **THEN** they are 0, 1/2, 2/3, 3/4, 4/5, 5/6, 6/7 and 7/8

### Requirement: Planar vertex streams
A vertex buffer SHALL reserve position (12 bytes per vertex), then, as the format word's bits ask, texture coordinates, colour (4 bytes), normal, tangent (four floats), morph delta (three floats) and a second texture stream (two floats), each stream holding every vertex's values back to back. Index batches SHALL use 1-, 2- or 4-byte indices (encoded 1, 2, anything else) in a CPU storage of a 12-byte header and per batch a mode and count (8 bytes) followed by its indices.

#### Scenario: Stream offsets
- **WHEN** a format asks for position and texture coordinates for 10 vertices
- **THEN** the texture stream starts 120 bytes after the position stream

### Requirement: Post-chain fades
The screen fade SHALL run 5000 ticks with alpha `1 - (start + 5000 - now)/5000` in double precision, its desaturation the base times that alpha and its sharpen the alpha squared raised to the tenth. The desaturation fade SHALL run 3000 ticks with ramp `1 - (start + 3000 - now)/3000`; below a quarter the mask is `1 - 4 * ramp`, else `(ramp - 0.25) * 1.333`, and the pass receives the inverse; a non-zero kind adds `(1 - mask)^2 * 0.4` to the bloom.

#### Scenario: The end of the desaturation fade
- **WHEN** the ramp reaches 1
- **THEN** the mask is 0.75 * 1.333 = 0.99975 and the pass receives 0.00025

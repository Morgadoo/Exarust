# Spec Delta

## ADDED Requirements

### Requirement: Blue-first and ZYX uncompressed orders
The decoder SHALL classify the words `0x0200C600`, `0x0200E608`, `0x1201C600` and `0x1201E608` as BGR8, BGRA8, ZYX8 and ZYXV8 and convert them to RGB8, RGBA8, XYZ8 and XYZV8 by swapping the first and third bytes. The three-channel orders SHALL give grey as the plain luma of the stored bytes without the swap, and the four-channel orders SHALL give alpha but no grey.

#### Scenario: A BGR pixel
- **WHEN** a BGR8 texel holds the bytes 10, 20, 30
- **THEN** its RGB8 value is 30, 20, 10

### Requirement: Registered compressed variants
Compressed words whose block layouts the game does not ship SHALL decode through the converter their registered fallback names. `DXT1a` SHALL make the three-colour mode's index-3 texel transparent black, `ALPHATC` SHALL read a BC4 plane as alpha with white colour, `LATC` SHALL decode the colour block in the second half of each 16-byte block, and `DXT5n` SHALL decode as plain BC3.

#### Scenario: A DXT1a block in three-colour mode
- **WHEN** a block whose first endpoint is not above the second holds an index-3 texel
- **THEN** that texel decodes to (0, 0, 0, 0) under DXT1a and to opaque black under the RGB variants

### Requirement: Block arithmetic
The block decoders SHALL use the original's integer arithmetic: bit replication for 5-6-5 endpoints and truncating divisions with no rounding term.

#### Scenario: A BC1 interpolant
- **WHEN** a four-colour block blends two endpoint channels at a third
- **THEN** the blend is the truncating integer division, not a rounded one

### Requirement: Texture unit bind cache
A texture bind SHALL do nothing when the same object already sits on the unit. A different object SHALL make its unit active when it is not, bind its target and name, and set the LOD bias parameter to zero unless the word has bit `0x200`. Binding nothing, or an object with no name, SHALL only clear the unit's entry. The active unit SHALL start at 0.

#### Scenario: A repeated bind
- **WHEN** the same texture is bound twice on one unit
- **THEN** the second bind performs no operation

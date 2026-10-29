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
The LOD bias SHALL be the base times (the supersampling factor times the single-precision third), evaluated in that order, where the factor is the supersampling level below 2 and that level less one from 2 up. Every material path, painted, race-skin and run-time apparel included, SHALL take the bias, and a texture settings change SHALL rewrite the bias of every loaded material.

#### Scenario: Supersampling factor
- **WHEN** the supersampling level is 1, 2 and 4 at a bias base of −0.75
- **THEN** the biases are −0.25, −0.25 and −0.75

#### Scenario: A quality change
- **WHEN** the bias changes to −0.25 with two materials loaded, one already at −0.25
- **THEN** both read −0.25 and one changed

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
A diffuse map stored as BC4, the original's luminance format, SHALL be uploaded as one channel and shown as grey on every material path, with no diffuse bias for a record that has a secondary colour. A BC4 specular or emissive map SHALL be shown as grey too.

#### Scenario: A BC4 diffuse
- **WHEN** a run-time apparel or race-skin material takes a BC4 diffuse
- **THEN** the shader repeats the red channel as grey

#### Scenario: A BC4 specular map
- **WHEN** a material's specular map is BC4
- **THEN** the material carries the grey specular flag and the shader repeats the red channel

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

### Requirement: Cube uploads
An image whose creation word picks the cube target SHALL upload six faces with a cube view, each face from the detail level with its stored levels back to back, or its generated chain when it stores one level. A block-compressed cube SHALL be refused.

#### Scenario: A two-level cube
- **WHEN** a 2×2 RGB8 cube with two stored levels is uploaded at detail 0, and at a detail that asks for level 2
- **THEN** it has six layers of two levels, and then six 1×1 layers holding each face's smallest level

### Requirement: Image reader dispatch
A name without a dot SHALL be read as a provider's image, by its magic. A name with a dot SHALL go to the reader registered for the text after its last dot, compared without ASCII case; the only reader SHALL be "rfi". An extension longer than 16 bytes, or a trailing dot, SHALL leave the extension empty, which matches no reader. A name with no reader SHALL report "Unrecognised file extension: " and the extension's first 15 bytes in the name's case, and SHALL load the grey placeholder.

#### Scenario: An unknown extension
- **WHEN** the name a.b.TGA is loaded
- **THEN** it reports "Unrecognised file extension: TGA" and loads the placeholder

#### Scenario: A leading dot
- **WHEN** the name .rfi is dispatched
- **THEN** its extension is rfi

### Requirement: Terrain texture path
The terrain's material textures SHALL be taken from a live record when one exists and otherwise queued for the texture loader behind the 1×1 placeholder with the streaming status alone, never decoded by the load step; a name the game does not ship SHALL give no texture. A terrain tile SHALL be spawned only once its materials' textures have arrived.

#### Scenario: A queued terrain texture
- **WHEN** a terrain material's diffuse map is asked for while no record holds it
- **THEN** a load is queued behind the placeholder and the tile waits until its upload replaces it

### Requirement: Waiting texture finished at once
A caller that loads the level in one call SHALL finish a record still holding its placeholder at once, with the word the record was created with and without taking another reference.

#### Scenario: Finished on the spot
- **WHEN** a record holding its placeholder is finished at once
- **THEN** the real image is under the same handle, the queued load is dropped and the record keeps one reference

### Requirement: Fast-load flag
With the texture manager's fast-load flag set, a load the material path queues SHALL start as the loader's priority item, so its decode does not wait for the upload pass; the terrain path SHALL ignore the flag. The flag SHALL be off, as the shipped game never sets it.

#### Scenario: The flag set
- **WHEN** the flag is set and a material texture and a terrain texture are queued
- **THEN** the material's load is the priority item and the terrain's is not

# Images (`.rfi`)

Status: **verified**. Raw storage (BC1/BC3/BC4/BC5 and uncompressed) and
packed (RLE) storage are decoded by `exa_formats::rfi`, written to PNG with
`exa-tool png` and checked by eye; the channel order of uncompressed images
was established by reverse engineering (below).

## Header (32 bytes)

| off | field | notes |
|---|---|---|
| 0 | magic | `C6 3D 2D 1D` |
| 4 | width | |
| 8 | height | non-square common (2048x256) |
| 12 | depth | 1; 0 on a handful of entries (treated as 1) |
| 16 | format word | bit fields, below |
| 20 | flags | e.g. `0x20AB0000`, `0x24AF0000`, `0x40A80000`; sampler/usage bits, partly decoded (below); `0x40000000` under the mask `0x70000000` marks a cube |
| 24 | storage | `0x10000000` raw, `0x50000000` packed (RLE; bit 30 is the test) |
| 28 | data size | bytes of the unpacked mip chain |

Flags: the original's texture loader keeps the word's bits from `0x10000`
up and adds its own. Bits `0x10000`, `0x20000` and `0x40000` repeat the
image across its width, height and depth; without them it clamps to its
edge (established by reverse engineering of the loader and its wrap
setup). Flag bits 28–30 reading `100` (`flags & 0x70000000 ==
0x40000000`) mark a cube: the image holds six faces, each with its own
mip chain, face after face, and `data size` counts one face (the
original's size, copy, create and conversion routines all multiply by six
on that test). `genref01r`, the indoor environment cube, is six 256² RGB8
faces of nine levels each, `data size` 262 143 (pinned by a corpus test).
`exa_formats::rfi::RfiHeader::is_cube` and `Rfi::decode_faces_rgba8` read
it. `BldGr02D`, the weapon blood gradient, has `0x20A90000`: it
repeats across and clamps along its height (pinned by a corpus test in
`exa_character`). `BldPrt02`, the blood droplet atlas, has `0x20A80000`,
and its built-in material loads it with `0xF0` (no wrap bit either): it
clamps on both axes (pinned likewise). The other bits are not decoded.

Raw images store the mip chain largest-first right after the header; the
chain is either complete (down to 1x1) or only the base level. The mip count
is derived from `data size`.

Rows are stored bottom-up (OpenGL origin): the human skin atlas decodes
upside-down when written top-down.

## Format word

```
bit 31        block compressed
bits 24..30   0x01/0x11 uncompressed, 0x81/0x82/0x92 compressed (sub-variants unknown)
bit 22        16-byte blocks (else 8-byte)
bits 8..15    channel mask: 0x20 alpha, 0x42 luminance, 0x84 two channels, 0xC6 RGB
bits 0..7     0x08 when alpha present
```

| word | count | format | typical use |
|---|---:|---|---|
| `0x813B4200` | 4600 | BC4 (LATC1) | grayscale apparel/weapon diffuse tinted by material, gloss |
| `0x927B8400` | 2300 | BC5 (LATC2/RGTC2) | normal maps (X, Y; Z reconstructed) |
| `0x813BC600` | 1390 | BC1 | diffuse |
| `0x823BC600` | 187 | BC1 | specular (`*s`) |
| `0x817BE608` | 110 | BC3 | diffuse with alpha (foliage) |
| `0x827BA408` | 4 | BC3 (guess) | |
| `0x813BE601` | — | BC1 with one-bit alpha (`DXT1a`) | registered by the original; not shipped |
| `0x813B2008` | — | BC4 alpha only (`ALPHATC`) | registered; not shipped |
| `0x817B6208` | — | two-channel BC4 luminance-alpha (`LATC`) | registered; not shipped |
| `0x827BE608` | — | BC3 data (`DXT5n`) | registered; not shipped |
| `0x0200C600`, `0x0200E608` | — | BGR8, BGRA8 | registered; converted to RGB8/RGBA8 |
| `0x1201C600`, `0x1201E608` | — | ZYX8, ZYXV8 | registered; normal-map channel orders |
| `0x0100C600`, `0x0111C600`, `0x1111C600` | 63 | RGB8 | reflection cubes, glow maps, a few diffuse maps and UI icons (`0100`); packed specular (`0111`); normal maps and one packed specular (`1111`) |
| `0x0100E608` | 23 | RGBA8 | foliage, fire particles, UI pictures |
| `0x01004200`, `0x01114200` | ~25 | L8 | height maps (`*h`) |
| `0x01006208` | 85 | LA8 | emblems |
| `0x01002008` | ~25 | A8 | cobwebs, font |

Uncompressed formats store channels in **R, G, B(, A)** order, in both
storage modes. Established by reverse engineering of the original's texture
upload (REA): its format table uploads `0x0100C600` as `GL_RGB` and
`0x0100E608` as `GL_RGBA`, both unsigned bytes; words with bit 20 set and
bit 31 clear (`0x0111C600`, `0x1111C600`) are looked up under the mask
`0x800CFFFF` and so share the RGB8 entry; and the packed-storage unpacker
copies each pixel byte for byte. The corpus agrees (`exa-tool corpus
images`): all 42 RGB8 normal maps hold z (about 255 on flat areas) in their
third byte, and foliage, fire particles, the tutorial pictures and the
embers' glow map (`embersael`) have natural colours only in this order. So
does a capture of the original's main menu: its title (`maintitle01`,
packed RGBA8) is steel blue, blue above green above red, as it decodes red
first. The community notes call these formats BGR and BGRA; that is wrong
for 0.9.5.

## Image sizes from the format word

The original sizes an image from the word's bit fields rather than a
table (`exa_formats::rfi::format_level_size` for one level,
`exa_formats::rfi::image_size` for one face):

- bit 31 set: block compressed. A block takes `1 + bits 19–23` bytes (8
  for BC1/BC4, 16 for BC3/BC5) and is `1 + bits 16–18` texels a side (4),
  so a level is whole blocks across and down;
- bit 18 set: bit-packed. The bits per pixel are bits 6–12 plus bits 0–5
  (1 for LUM1, 24 for RGB8, 32 for RGBA8) and a level is
  `ceil(width·height·bits / 8)`;
- otherwise byte-aligned pixels: `width·height·ceil(bits / 8)`.

A face is the base level, plus, when the flags carry the chain bit
(`0x80000`), every halved level while `width + height > 2` (a side of one
stays one). The face size never includes the cube's six. The pixel count
the converters walk (`exa_formats::rfi::image_pixels`) is the same walk
over `width·height`, times six for a cube. A run-length encoded image
unpacks to `data size` bytes. Established by reverse engineering of the
size, pixel-count, decoder and copy routines (REA).

The original does register blue-first words in a second table:
`BGR8` (`0x0200C600`) and `BGRA8` (`0x0200E608`) convert to `RGB8` and
`RGBA8` by swapping the first and third bytes, and the normal-map orders
`ZYX8` (`0x1201C600`) and `ZYXV8` (`0x1201E608`) convert to
`XYZ8`/`XYZV8` the same way. No shipped image uses any of the four; the
upload table lists their GL formats as `GL_BGR`/`GL_BGRA`. Exarust
classifies them and decodes them through the converters.

The compressed words whose block layouts are not shipped decode through
the registered converter family: `DXT1a` keeps the second mode's black
texel transparent, `ALPHATC` is BC4 alpha only, `LATC` takes two BC4
channels as luminance and alpha, and `DXT5n`/`DXT5na` decode as plain BC3
(the latter is a guess; only four images carry the word). The BC1 colour
expansion and the BC4/BC3 alpha blends use the original's integer
arithmetic: bit replication for 5-6-5 endpoints and truncating `/3`,
`/7` and `/5` divisions with no rounding term.

## Pixel-format conversion

The format word is also the original's in-memory pixel format id. Bits
6–12 count a pixel's colour bits, bits 0–5 its alpha bits, bit 18 marks a
bit-packed format and bit 31 a block-compressed one, so RGB8 is 24 bits,
LUMA8 16 and LUM1 1 (`exa_formats::pixel`). The original registers 20
formats, each with a fallback and a converter, in two groups:

| id | name | fallback | | id | name | fallback |
|---|---|---|---|---|---|---|
| `0x0100C600` | RGB8 | RGB8 | | `0x0111C600` | ABC8 | RGB8 |
| `0x01004200` | LUM8 | RGB8 | | `0x0111E608` | ABCD8 | RGBA8 |
| `0x0100E608` | RGBA8 | RGBA8 | | `0x01114200` | VAL8 | LUM8 |
| `0x01006208` | LUMA8 | RGBA8 | | `0x01116208` | VALA8 | LUMA8 |
| `0x01002008` | ALPHA8 | RGB8 | | `0x01118400` | AB8 | `0x01008400` |
| `0x01044040` | LUM1 | LUM8 | | `0x11118400` | XY8 | `0x01008400` |
| `0x1111C600` | XYZ8 | RGB8 | | `0x1111E608` | XYZV8 | RGBA8 |
| `0x0108D800` | RGBf | RGB8 | | `0x0109D800` | XYZf | XYZ8 |
| `0x0108F820` | RGBAf | RGBA8 | | `0x01084800` | Lf | LUM8 |
| `0x01082020` | Af | ALPHA8 | | `0x01094800` | Vf | VAL8 |

A converter reads one source layout and writes a target or declines. Most
compare the target under `0x800CFFFF`, which drops the data-kind bits:

- grey (LUM8, VAL8): copied to grey or alpha, (l, 255), (l, l, l),
  (l, l, l, 255), or to Lf as l × 1/255;
- alpha: copied, (a, a, a), (255, 255, 255, a);
- grey-alpha (LUMA8, VALA8; targets compared whole): alpha, (l, l, l),
  (l, l, l, a);
- two channels (XY8, AB8; compared whole): the second byte, (x, y, 255),
  (x, y, 255, 255), and for XYZ8 (x, y, z) with z rebuilt as
  √(1 − x² − y²), x and y centred on 127.5 and scaled by 1/127.5, mapped
  back by × 127.5 + 127.5, or 0 when 1 − x² − y² is not above 1.19 × 10⁻⁷;
- one bit (LUM1, lowest bit first): 0 or 255, to grey or RGB;
- RGB (RGB8, XYZ8, ABC8): luma 0.299 R + 0.587 G + 0.114 B in double
  precision, (luma, **1**) for LUMA8 (the original writes an alpha of 1),
  (r, g) for two channels, (r, g, b, 255);
- RGBA (RGBA8, XYZV8, ABCD8): (r, g, b) for any RGB target, alpha, luma,
  (luma, a);
- RGBf and RGBAf: the clamped float-to-byte below to RGB8 or RGBA8, the
  signed one to XYZ8 or XYZV8;
- Lf and Af: v × 255 with 0 only below 0 and 255 only above 1, to grey or
  RGB;
- Vf: the clamped float-to-byte to grey; to RGB without any clamp
  (`(v − 0) · 255/(1 − 0)`, keeping the low byte).

Floats come from bytes as b × 1/255 (the stored f32 `0x3B808081`). A float
goes back to a byte as 0 unless it is above 0, 255 unless it is below 1,
else v × 255 rounded. The signed form is v × 127.5 + 127.5, bounded by −1
and 1. Every rounding is the processor's default, to nearest with ties to
even. NaN converts to the integer indefinite value, so its byte is 0.

Exarust's decoder runs the stored format's converter to RGBA8 for grey,
alpha, grey-alpha and RGB images and copies RGBA images, which matches
what the original's RGBA8 targets give. Established by reverse engineering
of the two registration routines and every converter (REA).

The conversion of a whole image to another word walks the same
registrations in order (`exa_formats::rfi::ImageData::convert`). The same
word is a no-op. A requested word with bit 20 set and bit 31 clear whose
layout matches the image's under the mask `0x8F0CFFFF` only replaces the
word. Otherwise the first registered format whose id is the image's word
converts it: a request of 0 takes that format's fallback, an image word
with bit 20 set and bit 31 clear and the target's layout only takes the
target word, and anything else runs the converter over every pixel of
every face and stored level; the image's size becomes the target's and
its chain flag stays. An unregistered word or a declined target leaves the
image as it was. The original also registers the block-compressed
formats, whose converters Exarust does not have.

## Upload formats

The original's compatible-format table has seven uncompressed entries, each
a stored word compared under `0x800CFFFF` (which drops bit 16 and the
top-nibble kind) with a linear and an sRGB internal format, a GL format and
a type:

| stored word | format | linear | sRGB |
|---|---|---|---|
| `0x0100C600` | RGB8 | `GL_RGB8` | `GL_SRGB8` |
| `0x0100E608` | RGBA8 | `GL_RGBA8` | `GL_SRGB8_ALPHA8` |
| `0x01004200` | LUM8 | `GL_LUMINANCE8` | `GL_SLUMINANCE8` |
| `0x01002008` | ALPHA8 | `GL_ALPHA8` | `GL_ALPHA8` |
| `0x01006208` | LUMA8 | `GL_LUMINANCE8_ALPHA8` | `GL_SLUMINANCE8_ALPHA8` |
| `0x01008400` | AB8 | `GL_RG8` | `GL_RG8` |
| `0x11118400` | XY8 | `GL_RG8` | `GL_RG8` |

So the packed `0x0111`/`0x1111` words share the RGB8 row, which has an sRGB
variant, but they are still uploaded linear: the status word's data bit
decides. ALPHA8 and the two-channel words have no sRGB variant at all, so
they are never sRGB-decoded. The compressed words map to their block
formats; BC1 and BC3 have sRGB variants, BC5 is always linear, and BC4
(a single grey channel; 4 621 of the shipped images, the apparel and item
diffuse and gloss maps) is uploaded as the luminance LATC format, also
without an sRGB variant.
Exarust uploads BC4 as `Bc4RUnorm` and repeats its red over the colour in
the surface shader, so both the value and the format match.

## Generated mip levels

When the original builds a mip chain itself (single-level uncompressed
images), it turns the base into floats, filters every level from them
(`exa_render::mipmap`) and turns each level back into bytes. Colour
images use a bank of 13 normalised Kaiser kernels: level `L` has
`8·2^(L−1)` taps and a sinc scale of `2^−L`, with `I0(4·√(1 − t²))/I0(4)`
as its window, the weights summed in double precision. Data images
(format word bit 16 set, bit 31 clear: the `0x0111` and `0x1111` words)
use a second, uniform bank instead: `2^L` taps of `2^−L`, a plain box.
Each axis wraps by the image's own header flags (`0x10000` across,
`0x20000` along the height) and clamps to the edge without them. Each
level shifts the sides right with a floor of one, so non-power-of-two
images are mipmapped as well, and the chain stops after the level whose
two sides sum to two (a 1×1 level is filtered and kept). Established by
reverse engineering of the unit's two bank builders, its chain driver and
the texture loader's call into it (REA). The format word picks the
conversion:

- bit 28 set (signed data: XYZ8, XY8, XYZV8): centred, (b − 127.5)/127.5,
  and back by the signed form;
- bit 16 or 17 set (VAL8, ABC8, ABCD8 and the other `0x0111` words):
  linear, as above;
- otherwise colour is filtered **squared**: (b/255)², back through √ and
  × 255. RGBA8 and LUMA8 keep alpha linear. Other single, two- and
  three-channel words square every channel; their way back clamps on the
  float's raw bits (0 when they read as a non-positive integer, 255 from
  `0x3F7F7F6F`, just under 1, up).

A header flag of `0x0400_0000` marks vector data (normal maps): before
each filtered level of a three- or four-channel image is converted back,
every texel's first three channels go through the engine's vector
normaliser (a zero vector stays zero); one- and two-channel images are
left alone. The same flag applies to the resample-to-size path used for
icons, cursors, terrain inputs and GUI images. That path takes the
chain's level `k` when the destination is the source shifted down by `k`
(1–13) on both axes, and otherwise a separate phase-shifted window
(`(1 − t)³·(1 + 3t − 3t² − 13.6t³)`, two output texels each way), in a
space of its own: signed words centred, data words and magnified images
linear, other words squared, with RGBA8 and LUMA8 colour words squared
(linear alpha) whatever the sizes.

So a 0/255 checker halves to about 180, not 128, in a plain RGB8 or RGBA8
texture. A test for an exact `ALPHA8` word, which would select the linear
pair, can never match (the mask drops the bit it tests), so alpha-only
images are squared too.

Which stored level becomes the uploaded base follows `TextureDetail`
(header flags bits 22–23 are the image's detail class 0–3):
`first = max(0, class − TextureDetail + 1)`, then raised while the width,
and then the height, shifted by it is larger than the driver's largest
texture side (read before any upload). With the mip bit (`0x40`) in the
creation word, and in the texture quality mask, every stored level from
`first` is uploaded and a generated chain runs to its end; without it only
level `first` is uploaded (or generated), which halves the stored resolution
instead of changing the format. A compressed image without a stored chain
keeps its base level and its mip bits are cleared: the CPU cannot downsample
it. In the shipped data 8 616 of 8 839 images are class 2, 200 class 0, 2
class 1 and 21 class 3, so at the retail `TextureDetail = 3` only those 21
start at level 1 (established by reverse engineering of the upload routine;
counts by `exa_assets` `every_shipped_image_converts`).

The image's flag bits from 16 up join the requested word when a named
image is loaded: bits 16–18 repeat it on S, T and R. Linear or sRGB storage
is decided by the word's data bit (`0x100`) and, for an uncompressed image,
by format bit 16 (the `0x0111` and `0x1111` words), which the upload routine
treats as data; the compressed `0x82` and `0x92` words are data too. The
compatible-format table (below) adds that ALPHA8 and AB8 have no sRGB
internal format, so they upload linear whatever their bit 16 says.

## Packed storage (`0x50000000`)

Only uncompressed formats are packed: emblems, fonts, some foliage, small
textures. The whole mip chain is one run-length stream that unpacks to
`data size` bytes. With `bpp` bytes per pixel and a threshold of 64 when
`bpp == 1` (63 otherwise), each control byte `c` means:

- `c < threshold`: `c + 1` literal pixels follow;
- otherwise: one pixel follows and repeats `c - 61` times.

The codec was first described by the community translation projects (see
`../research/online-research.md`). Exarust's decoder is an independent
implementation, checked on fonts and emblems.

## Image copies and blank images

`exa_formats::rfi::ImageData` is the owned form the original's copy and
create routines work on. `copy_of` (the copy that keeps the chain) unpacks
a packed source first (the original logs "Source image is run length
encoded." and decodes) and copies `data size` bytes of each face, one
`data size` apart. `copy_base` (the copy without it) clears the chain flag,
sizes the base level and takes each face's base the same source
`data size` apart. `blank` is the create call: depth 1, the given flags and
word, and the image's size from the routine above (the chain included when
the flags ask), six faces for a cube. `render_target` is the variant the
icon renderer uses: flags with no detail class get class 2, and the pixels
sit in the header's block. `resize` sets `data size` and holds six times it
for a cube; `release` frees the pixels and leaves the header. Established
by reverse engineering of those routines (REA).

## Naming conventions

Suffixes: `d` diffuse, `n` normal, `s` specular, `g` gloss, `h` height,
`m1`/`m2` masks, `o` opacity, `r` reflection. Character skin sets:
`humandiff`, `humandifm`, `humannrmf0..4` / `humannrmm0..5` (female/male
normal variants), `humanspcf/m`, `humandcmp*` (decomposition/injury masks).

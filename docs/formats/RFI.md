# Images (`.rfi`)

Status: **verified**. Raw storage (BC1/BC3/BC4/BC5 and uncompressed) and
packed (RLE) storage are decoded by `exa_formats::rfi`, written to PNG with
`exa-tool png` and checked by eye.

## Header (32 bytes)

| off | field | notes |
|---|---|---|
| 0 | magic | `C6 3D 2D 1D` |
| 4 | width | |
| 8 | height | non-square common (2048x256) |
| 12 | depth | 1; 0 on a handful of entries (treated as 1) |
| 16 | format word | bit fields, below |
| 20 | flags | e.g. `0x20AB0000`, `0x24AF0000`, `0x40A80000`; sampler/usage bits, not decoded |
| 24 | storage | `0x10000000` raw, `0x50000000` packed (RLE; bit 30 is the test) |
| 28 | data size | bytes of the unpacked mip chain |

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
| `0x0100C600`, `0x1111C600` | ~40 | RGB8 | reflection maps, small normals |
| `0x0100E608` | ~25 | RGBA8 | |
| `0x01004200`, `0x01114200` | ~25 | L8 | height maps (`*h`) |
| `0x01006208` | 85 | LA8 | emblems |
| `0x01002008` | ~25 | A8 | cobwebs, font |

Uncompressed formats store channels in **B, G, R(, A)** order.

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

## Naming conventions

Suffixes: `d` diffuse, `n` normal, `s` specular, `g` gloss, `h` height,
`m1`/`m2` masks, `o` opacity, `r` reflection. Character skin sets:
`humandiff`, `humandifm`, `humannrmf0..4` / `humannrmm0..5` (female/male
normal variants), `humanspcf/m`, `humandcmp*` (decomposition/injury masks).

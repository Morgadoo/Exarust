## Context

`exa_formats::rfi` decodes every `.rfi` image to RGBA8 for the CPU paths
(paint, terrain, `exa-tool png`) and for every texture that is not uploaded
block-compressed. Uncompressed colour images (RGB8 `0x0100C600`,
`0x0111C600`, `0x1111C600`; RGBA8 `0x0100E608`) were read as B, G, R(, A),
following the community notes. The embers' glow map came out blue, and the
question was whether the decoder or the material's use of the map was
wrong.

The format page this change extends is
[`docs/formats/RFI.md`](../../../docs/formats/RFI.md) (Format word, channel
order). Research notes with addresses stay git-ignored under `context/`.

## Goals / Non-Goals

**Goals:** decode uncompressed colour images in the original's channel
order, raw and packed; pin it with a test; correct the docs.

**Non-Goals:** a new capture of the running original at the fire pits; the
UI and powers that would draw the thaumaturgy icons; any other emissive
behaviour.

## Decisions

- **Stored order, red first, for every uncompressed colour image.** The
  original's upload looks the format word up in its texture format table and
  passes that entry's pixel format to `glTexImage2D`: `GL_RGB` for RGB8,
  `GL_RGBA` for RGBA8, unsigned bytes. The table has no BGR or BGRA entry.
  The packed-storage unpacker copies 3- and 4-byte pixels unchanged, so raw
  and packed images upload the same way. The decoder now emits the stored
  bytes as R, G, B(, A).
- **No change to the embers' material.** The record (`embersa_low`) is lit
  and specular (flags `0xF`), kind 1, with diffuse `EmbersAD`, emissive
  `EmbersAEL` and opacity `EmbersAD` layers and an emissive colour of 0.588
  grey. The original samples the emissive map's RGB times that colour; the
  map is orange once read in the stored order. Exarust already draws the
  emissive slot this way.
- **The environment cube keeps its own reader.** `cube_image` already read
  `genref01r`'s faces red first, so indoor ambient and reflections do not
  change.

## Open reverse-engineering questions

- How does the original upload RGB8 for raw and for packed storage?
  *Answered (REA):* format table entry {RGB8, sRGB8, `GL_RGB`, unsigned
  byte}; the same call for both storage modes, after the unpacker.
- Does the unpacker or the loader reorder channels? *Answered (REA):* no;
  pixels are copied byte for byte, and the upload passes the buffer as is.
- How do the data-flagged words (`0x0111C600`, `0x1111C600`) upload?
  *Answered (REA):* words with bit 20 set and bit 31 clear are compared
  under the mask `0x800CFFFF`, so they share the RGB8 entry.
- How does the embers' material sample `EmbersAEL`? *Answered (data and
  the material notes):* as the emissive map, RGB times the record's
  emissive colour, without a gradient or tint.
- Which images change? *Answered (corpus, `exa-tool corpus images`):* all
  63 RGB8 and 23 RGBA8 images in the retail packs. The 42 normal maps hold z
  in the third byte (none with red and blue swapped); foliage, fire
  particles and the tutorial pictures look natural only in the stored order.
- Does the running original agree? *Answered (capture):* the main menu's
  title (`maintitle01`, packed RGBA8) is steel blue in a screenshot of the
  original, blue above green above red, as it decodes red first.

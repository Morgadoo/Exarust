## Context

Exarust paints apparel on the CPU (`exa_assets::paint`) and uploads three
maps per item: colour, surface (gloss, metal) and normal. The colour map went
up as sRGB, the others as linear data, each with a mip chain averaged from
the stored bytes. The braies of the underwear change render darker than the
original's, and a probe that uploaded the colour as linear data looked
closer. This change establishes the original's colour chain before touching
the upload.

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Object database
items and apparel painting, new paragraph Colour space). Research notes with
addresses are git-ignored (`context/re/painter-colour-space.md`).

## Goals / Non-Goals

**Goals:** state the original's colour chain for painted apparel and keep
Exarust on it for every finish; make the encodings explicit at the painter;
pin the chain with a test.

**Non-Goals:** the skin composite, a run-time trace of the original, any
finish's maths.

## Decisions

- **Keep the colour map sRGB, with the finishes' values as texels.** The
  original paints into sRGB colour targets with sRGB encoding off. Its scene
  samples them with sRGB decoding and multiplies them by the painted
  material's diffuse colour of 1. Every input is loaded as data, and the
  palette colour is plain HSV to RGB. Exarust's painter reads bytes/255 and
  the plain HSV colour and uploads its output as sRGB, which is the same
  chain. A linear upload would turn the finishes' sRGB values into albedo and
  brighten everything painted. The probe's closer look comes from the skin,
  which lacks the original's composite.
- **Average sRGB mips in linear light.** The original's paint targets are
  mipmapped by the GPU, which treats sRGB levels in linear light, as it does
  when filtering ([inf] for the driver's mip generation). Exarust decodes
  the four texels through a table, averages them and encodes the mean by
  thresholds, so flat areas keep their texel exactly. Alpha and the linear
  maps are averaged as stored.
- **Encodings live with the painter.** `Painted::COLOUR`, `SURFACE` and
  `NORMAL` name each map's encoding, and the upload follows them, so the
  chain is stated once next to the finishes that depend on it.

## Open reverse-engineering questions

- Render targets of a paint job, and whether sRGB encoding is on while
  painting. *Answered (REA):* colour sRGB RGB8 (sRGB RGBA8 for items with
  alpha), specular linear RGB8, normal linear RG8, shade scratch R8. Both
  the item and the character jobs switch sRGB encoding off first, through
  the engine's cached setter, which calls `glDisable`. The loader's bindings
  confirm the entry point.
- How the paint programs sample their inputs. *Answered (REA):* shade maps,
  masks and item normals load with the data flag. All 181 library-texture
  loads of the 25 procedural materials carry it, and the upload picks the
  linear format for data. The palette colour reaches the uniform without a
  transfer function.
- How the scene samples the painted texture. *Answered (REA):* the painted
  material is a run-time template with diffuse 1, packed specular and the
  three targets. The PBR shader multiplies its diffuse by the sampled colour
  and by 1 − metal. Nothing skips sRGB decoding or copies the targets into
  another format.
- Does the retail frame carry an extra global encode (an NVIDIA-only switch
  in the window copy)? *Answered (screenshot measurement):* no. The braies'
  hue under the torch (G/R 0.31) matches a single encode (0.40 expected),
  not a double one (0.68).
- Are Exarust's braies darker than the original's, measured against
  something other than the skin? *Answered (screenshot measurement):* no.
  Normalised by the lit floor beside the feet, the original's braies are
  0.38/0.41/0.49 (linear R/G/B) of the floor and Exarust's about 0.6–0.7.
  In numbers the original's braies are darker than its skin too; they read
  pale because they are less saturated than the orange skin and rock. The
  figures are rough (torch flicker, different floors and light geometry), but
  a linear upload, about 3× brighter, would overshoot them.
- Do the paint targets hold the programs' output at run time, or does the
  driver encode them anyway? *Open:* the static code says the output. No
  path in the binary enables sRGB encoding during either paint job. A GL
  trace or read-back of one paint job would confirm the driver's side; the
  retail game was not run for this change.

## Risks / Trade-offs

- The braies stay darker than the skin until the skin composite exists. The
  mismatch now has an identified owner instead of a wrong fix.
- Linear-light mips brighten painted items at a distance (braies +11 %,
  mail +49 % at mip 4). Close views are unchanged.
- Painting costs one table lookup per mip texel more. The thresholds are a
  binary search over 255 values.

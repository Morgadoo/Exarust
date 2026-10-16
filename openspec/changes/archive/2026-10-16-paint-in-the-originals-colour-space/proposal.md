## Why

The underwear change left one question open. The braies render slate grey
and darker than the skin under the torch, while in the original's
screenshots they read as pale grey linen next to darker, more saturated
skin. A probe that uploaded the painted colour as linear data instead of
sRGB looked closer, and pointed at the painter's colour space.

Reverse engineering of the original's paint jobs settles the chain from the
painter's inputs to the value the scene lights:
- the finishes read the shade map, masks and library textures as stored;
- they take the palette colour as computed;
- they write into an sRGB colour target with sRGB encoding switched off;
- the scene decodes that target when it samples it.

Exarust's painter and upload already follow this chain. Uploading the colour
as linear data would contradict the original. The one step Exarust does
differently is the mip chain. The original's sRGB targets get their mips from
the GPU, in linear light. Exarust averaged the stored sRGB bytes, which
darkens painted items seen from a distance: at mip 4 the braies lose 10 %
of their brightness and mail 33 %.

Phase 3 (Bodies), milestone M3 "standing man", which calls for a fully
dressed character walking a level.

## What Changes

- The painter states how each painted map is read: colour as sRGB, surface
  and normal maps as linear data. The upload takes the encoding from it
  instead of literal flags.
- The mip levels of a painted colour map average colour in linear light, as
  the original's sRGB targets do. Alpha and linear maps are averaged as
  stored.
- A test pins the chain: palette colour 1 on a mid-grey shade map ends at
  the expected texel and sampled value, for a plain and a cloth finish.
- `docs/formats/OTHER.md` documents the painter's colour space.

Evidence:
- reverse engineering of `Exanima.exe` 0.9.5 through REA (the item and
  character paint jobs, their render targets, the sRGB-encoding switch, the
  load flags of the shade maps, masks and library textures, the palette, the
  diffuse pass's uniforms, the run-time painted material and the scene
  shader's diffuse term);
- the original's cloth program evaluated independently for the test's
  inputs;
- measurements of the original's screenshots (braies against skin and
  against the lit floor, and the braies' hue under the torch).

Out of scope:
- The skin composite (`chargen_*`), which darkens the original's skin to
  0.29–0.64 of its base before the scene decodes it. Its absence makes
  Exarust's skin several times too bright next to painted cloth. That is the
  main remaining cause of the braies-and-skin mismatch.
- A GL trace of the running original to confirm the paint targets' contents
  at run time.
- Every other painted finish's maths.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `apparel`: painted colour is read in the original's colour space, and its
  mip levels average in linear light.

## Impact

`exa_assets` (`paint`: the maps' encodings; `model`: upload and mip chain),
`docs/formats/OTHER.md`, `docs/PLAN.md`.

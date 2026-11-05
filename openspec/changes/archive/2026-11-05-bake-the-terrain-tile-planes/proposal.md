# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the terrain paragraphs. The
terrain's two read-back passes were modelled but never run: the tile's mask
pass (the plant density) and height pass (the ground's detail lift) had a
CPU stand-in that sampled each material's height and mask maps per vertex
in world metres with `+v`, at the base level, sharpened the height pass's
opacities with the blend masks, built opacities from float weights, and
stored the density without the target's sRGB encode. The frame hook's bake
state (bit 0x1000) was read as a "reload" that hid the tile, and the
hand-off that copies the planes back had no caller.

## What Changes

- The tile bake runs the original's mask and height passes texel by texel
  with the GPU's rules: the root node's quads (240² and 244², the height
  quad reaching 241 texels and the rest keeping its 0.5 clear), the merged
  weight bytes filtered bilinearly, the programs' arithmetic (layer 0 of
  the first group ignores its weight; the height pass has no mask
  sharpening and boosts the weight by 1.5; `clamp` takes NaN to 0), groups
  of four with later groups blending `ONE, SRC_ALPHA`, each material map's
  own sampler (nearest texel and level, the bind's LOD bias, the level of
  detail from the quad's constant footprint) and 8-bit writes, the mask
  pass's sRGB-encoded.
- The record hand-off takes the planes by the manager's feature word: the
  244² plane becomes the vertex channel (`(byte − 127) × 1.25 × (2 − n.y)`
  cm), the packed plane's green the plant density on its 240² texels.
- The frame hook's bit 0x1000 is the bake (the worker sets it whenever the
  renderer reads planes back; the shipped manager reads both), 0x2000 the
  pending read-back the next frame takes, which sets the upload bit; a tile
  is inserted only after both. The bake no longer hides a tile.
- The tile mask and height pass descriptions blend only from the second
  group of four (the first overwrites).

## Evidence

REA on the bake driver (feature bits 0x1000/0x2000, the 244² working
target in the sRGB format, sRGB writes on for the mask pass and off for the
height pass, the read rectangles), both passes (uniform uploads: the mask
colour or height triple with alpha forced to 1, tiling, the stretched
sharpness only in the mask pass, mask bits only in the mask pass; the
blend-mode word switched at the group starting at layer 4), the blend-mode
helper (`ONE, SRC_ALPHA`; blending off for the base mode), the read-back
hand-off (height plane into the record, packed plane to the callback, the
state bits), the manager's constructor (feature word 0x3000), the worker
(bake bit when the word has 0x3000), the frame hook (bake, then read-back,
then insert) and the terrain material loader (the maps' word 0x70140, no
quality mask). Corpus: the shipped palettes' mask greens (Mat Grass 0.4 to
0.8, Mat GrassT 0.7 to 1).

## Out of scope

The passes do not run on the GPU (Exarust builds tiles on the async pool;
the CPU pass follows the GPU's rules, without the hardware's 8-bit filter
weights). Whether the driver linearises the mask plane on read-back is not
observable from the binary: the desktop GL rule (stored values returned)
is assumed. The composite set path stays opt-in; light-distance planes,
node insert and unload, and the priority slot's caller are unchanged.

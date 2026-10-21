# Proposal

## Why

The seam skinner had been rebuilt from a pseudo-code reading of the
original's builder, and the character skin tone and texture job rules were
added alongside it. Reading the builder instruction by instruction showed
that the reading was wrong in ways that tear the shipped bodies open: the
edge falloff follows the byte-2 marker class, not the seam class; the
position marks are shared by every group; the joint matching's scores and
its spare-group search differed. The tone grid had been recovered from
two-decimal published values although the original builds it from an HSV
table, and the texture job reads all eight appearance layers, not seven.

## What Changes

- Seam groups, their boundary edges, boxes and centres follow the
  original's trace, growth and box order; a position belongs to the first
  group that reaches it; apparel runs the body's groups before its marked
  triangle groups.
- The body joint matching uses the original's keep-or-swap scores and its
  spare-group search with the stale threshold; the blend keeps the NaN and
  double-precision compares the original makes.
- The skin tone multiplier is computed from the original's HSV tone table,
  grid halves and fast power, bit for bit.
- The texture job reads all eight appearance layers; a missing mask still
  ends the list.

## Capabilities

### New Capabilities

### Modified Capabilities

- `motion-playback`: the skinned body and garments.
- `rendering`: the built-in skin's tone.
- `apparel`: the painted layers.

## Impact

- `exa_assets::skin` (builder, matching, weights), `exa_assets::human_skin`
  (`tone_colour`), `exa_formats::objdb` (`Appearance::used_layers`),
  `docs/formats/RSQ.md`, `docs/formats/OTHER.md`, `docs/RENDERING.md`, the
  `exa_assets` README.
- Characters look different in motion: 28 % of the skinned vertices of the
  shipped bodies and garments change weights, the human and ogre bodies no
  longer tear at their seams (533 and 517 mismatched seam pairs before, none
  now), and blends reach 5 cm where the marker class says so. Skin tones
  change by less than 0.006. Building every shipped body and garment's
  weights takes about 110 ms in all (87 ms before), a fraction of a
  millisecond per model.
- Evidence: REA (Ghidra) on `Exanima.exe`: the builder's trace, growth,
  box, matching, weight and share routines read as instructions (operand
  order, ordered and unordered compares, double compares, the corner
  tables), the engine's vector helpers (squared distance summed x, y, z;
  distance summed (x² + z²) + y²; lerp), the tone table's start-up build,
  the grid interpolation, the sRGB decode and encode with the fast
  logarithm and exponential, and the texture job's layer loop. Corpus: the
  shipped bodies and the 356 character models of the installed game.
- Out of scope: the merged `_SKIN` mesh and its cull bits, the group stack
  the apparel matching consults, the in-memory node flags that choose the
  joints and the apparel mode (still taken from the pack and bone names),
  the race flag that restricts some races to the men's skin textures, the
  layers' strength and jitter, and the decomposition pass.

## Phase

Phase 3, bodies: skinning, with the Phase 5 character look (skin tone and
apparel painting).

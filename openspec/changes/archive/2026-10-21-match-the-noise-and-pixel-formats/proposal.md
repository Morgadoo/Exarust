# Proposal

## Why

Exarust's terrain noise maps were an approximation: every texel used the
"below ½" blend of each octave, rounded differently and folded the ridge in
single precision at sample time. The original's noise unit also builds
ridged float images and an animated field for the force and spirit powers,
which Exarust lacked. Its pixel-format converters, the 20 registered formats
and the byte/float conversions the mipmapper picks per format word, were
reproduced only for the RGBA8 targets of the decoder. Generated mips were
filtered linearly, where the original squares plain colour first.

## What Changes

- The procedural noise of the original: the white-noise sequence (bytes and
  floats), byte octaves with both blend halves, the ridge fold, the ridged
  float octaves and the animated noise field (planes, phase, amplitude
  easing, row-by-row building, wrap).
- The terrain's three noise maps come from it, map A ridged once after its
  octaves.
- The 20 pixel formats with ids, fallbacks and sizes, and every converter
  between them, including luma, the rebuilt Z, the 1-bit expansion and the
  float formats.
- The `.rfi` decoder runs the stored format's converter.
- Generated mip chains are filtered in the space the format word picks:
  linear, colour squared (alpha linear for RGBA and grey-alpha) or
  centred.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: procedural noise images and the animated noise field; the
  filtering space of the Kaiser texture mips.
- `terrain`: the terrain noise maps.
- `texture-decoding`: pixel-format conversion.

## Impact

- `exa_formats::pixel` (new), `exa_formats::rfi`; `exa_render::image_noise`
  (new), `exa_render::mipmap`; `exa_assets::texture`;
  `exa_world::terrain::blend`.
- Docs: `docs/formats/RFI.md` (Pixel-format conversion, Generated mip
  levels), `docs/formats/TERRAIN.md`, `docs/RENDERING.md` (Procedural
  noise), the `exa_formats` and `exa_render` READMEs.
- Evidence: reverse engineering of `Exanima.exe` (REA, Ghidra): the noise
  unit's generators, octave passes and field update, the terrain texture
  renderer that calls them, the force power's set-up, the two format
  registration routines, every converter and the mip builder's choice of
  conversion. The numeric pins come from a separate model of the rules.
  No corpus fields change.
- Out of scope: drawing the animated field in the force and spirit powers;
  the worker thread the original generates noise on; the original's
  any-to-any conversion entry (the registry lookup and its fallback chain);
  the mipmapper's per-axis wrap flags and resize path; the BC codecs.

## Phase

Phase 2, static world (terrain blend rules and texture loading), with the
noise field for Phase 5's power effects.

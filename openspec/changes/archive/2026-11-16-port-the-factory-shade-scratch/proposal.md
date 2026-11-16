## Why

Roadmap task B4 (lane `texgen`, Phase B close): the generated textures'
unit routines (the procedural materials' factory paint job and the
characters' apparel texture job). The ledger left the factory job's shade
scratch missing (part flags 1, 2 and 4 ignored, texture-only overlay parts
never painted), the alpha job's colour target opaque, the apparel job's
library tiles unplaced and its wear table read by the drawn material.

## Evidence

REA (Exanima.exe 0.9.5), checked in the assembly where constants and
order matter:

- The factory job's builder lists a region for every part in recipe order
  whose choice byte is not zero (a choice past the variants takes the
  first), overlay parts included, keeping it when its first mask loads. It
  sets region flag 1 always, 4 for part flag 2, else 2 for part flag 1,
  and 8 for part flag 4.
- The job's render callback draws every region with a shade texture into a
  one-channel scratch of the atlas size (cleared to 1, nearest, clamped
  edges) with `shademapa` (`textures.flb`), blended ONE, SRC_ALPHA, `BMsk`
  (1, 0) for region flag 4, (1, 1) for 2, (0, 0) otherwise. Every pass's
  uniform set-up then reads `ShdTile` as the region's rectangle (its height
  times `UVScale` for the blade programs), so the shade comes from the
  scratch at the region's atlas texels. `steel_n` turns the item normal by
  `sign(ShdTile.xy)`.
- An alpha job (the factory flags it when an area rolls Gemstone) runs,
  after the colour passes, every layer's alpha program blended ZERO,
  SRC_COLOR; only `gemstone_a` exists in the shipped `textures.flb`. The
  colour passes write no alpha. The run-time material `_procmatr` differs
  from `_procmat1` only by the refraction mode (material word 0x360207).
- The apparel layer-list builder draws four tile offsets from the item's
  paint seed (the body word before the appearance block) into the block's
  tile fields, which every pass reads (`MatTile.zw`, `DmgTile.zw`); it
  reads the wear table by the stored material byte (entry 0 is 1); a set
  that names no material leaves material 0.
- Corpus (`exa-tool corpus factories`): 17 overlay parts (fullers, holes),
  24 parts flagged 2, one part flagged 4 (in a test recipe no item names).

## What changes

- `exa_assets::atlas`: the shade scratch (`shade_scratch`,
  `shade_scratch_texel`, `region_flags`, `shade_mask_weights`), every
  region's passes reading it, and the alpha job's alpha passes.
- `exa_assets::paint`: `ShadeTile` on `RegionJob`, the steel normal's turn,
  `gem_a` (`gemstone_a`) and `RegionJob::alpha`, `apparel_tint` by the
  stored byte, `layer_material` leaving 0 for an empty set.
- `exa_assets::weapon`: `paint_regions` (the job builder's list); the alpha
  job flag; `exa_assets::model`: the factory material refracts for an alpha
  job, the apparel job tiles from the seed.
- `exa_formats::objdb::Appearance::seed`, read from the item body.
- `exa-tool paint-bench` and the part flag tally in `corpus factories`.

## Out of scope

- Linear filtering of the source maps (the CPU painter samples nearest).
- The alternate-normal draw of part flag 4 (only the `shield test` recipe).
- The GPU job queue, surface cache and program objects (Exarust paints on
  the CPU and Bevy's task pool).
- The character creator's paint calls.

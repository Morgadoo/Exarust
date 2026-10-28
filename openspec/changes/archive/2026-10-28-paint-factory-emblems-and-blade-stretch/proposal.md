# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the factory paragraph of change
`correct-the-factory-roll-and-paint`: "Not in: the shared atlas,
texture-only overlay parts, emblem layers." The factory paint job also
binds a texture-only stretch (`UVScale`) that the blade programs read,
which Exarust did not apply.

## What Changes

- `exa_formats::factory::Factory::pattern_position`: the emblem pattern's
  position (`PtrnPos`) from the atlas ratio and the placing variants
  (flag `0x1000`), in the original's double/float mix.
- `exa_formats::factory::Part::uv_scale`: the region's `UVScale` from the
  last deform whose header word is 6.
- `exa_formats::procedural::emblem_picture`: emblem picture names, shared
  with `exa_game::emblem`.
- `exa_assets::paint::Pattern` and `Tiling::uv_scale`: a patterned layer
  paints only the colour pass, through its picture for painted-pattern
  materials; blade layers sample their mask, shade and region normal at
  (u, v·UVScale).
- `exa_assets::model::FactoryPaint` carries the emblem layers, `UVScale`
  and `PtrnPos`; `exa_assets::weapon` fills them for every factory weapon.

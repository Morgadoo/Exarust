# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items): the item factory and the
procedural materials. After `paint-factory-emblems-and-blade-stretch` the
painter still painted a factory part at its source texture's size, ignored
the region's width crop and the texture detail, rebuilt a weapon's meshes
on every condition change, and had no model of the material programs, the
pass uniforms, the paint-job ring or the surface cache.

Evidence: the six pass draws, the program compile, the region quad, the
job ring push and flush, the surface cache get and return, the factory
paint builder (crop = 1 − texture parameter, 1) and the weapon build's
impact-sound table, checked instruction by instruction against the
executable's assembly.

## What Changes

- `exa_assets::paint::RegionDraw::uniforms`: `MatTile`, `DmgTile`,
  `ShdTile` (with `UVScale` folded into its y in every pass), `MatCol`,
  `Damage`, `Dirt`, `Quality`, `PtrnPos` (painted pattern) and `NrmTile`
  (specular).
- `exa_assets::paint::paint_target_size` and `paint_sized`: a factory part
  paints at its atlas region's texel size, halved below texture detail 3.
- `exa_assets::paint::Tiling::crop`: columns past the region's width crop
  stay cleared; the source keeps its scale.
- `exa_assets::weapon`: a condition change repaints the cached geometry.
- `exa_assets::paint_jobs`: the job ring, the surface cache and its
  extra-layer pool, the shade scratch targets and the target formats.
- `exa_formats::procedural::programs` and `sampler_name`: the program
  variants a material compiles.
- `exa_formats::weapon_stats::impact_sound`: the factory's hit sound table.

## Out of scope

The shared factory atlas and the merged `_PROCITEM` mesh, the weapon's
rigid body, the `shademapa` mask gating (part flags 2 and 4), GPU program
compilation (Exarust paints on the CPU), and the never-selected
`_procmatr` material.

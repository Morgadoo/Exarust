## Why

Phase 5 (rendering) in docs/PLAN.md, the "Shading-mask GLSL snippets"
component (the smoke and mist draw) and the default material factory. The
smoke program was ported but nothing drew with it: the material plug-in
that routes records to it was thought to be registered at run time by an
unported effect. In fact the smoke unit's start-up hook rewrites the type
table's third slot, so every type-2 record (the wraith's skin and robe)
draws as smoke, while Exarust drew them as an unlit emissive surface.

## What the original does

REA and the disassembly of the smoke unit (start-up hook, key routine,
draw, program compile, cache free), the unit-initialisation table, the
type table and its reset, the default factory's render-state build, the
program compiler and the shader builder's quality word; the shipped
`shadesmk_f`, `normal` and `fog` text of `shading.flb`:

- The start-up hook (an entry of the unit init table) writes the smoke
  draw, the key routine and mask `0x400008` into type 2's slot of the
  stored table, which every reset copies.
- The key routine keeps the state words under `0x0003_C00F_0048_0008` and
  ORs the builder's quality word in; the draw adds `VrtCol`/`VrtDlt`,
  caches one program per object/environment pair, sets `UVOfst` and draws
  (once per sub-view under the supersampled passes).
- The compile fails silently (no message) when the expanded text uses an
  undeclared identifier: `vrNrm` without `UseNrm`, `vrCol` without
  `VrtCol`, and `amb`/`b` in the distance-fog branch of its fog include.
- Only `wraithskin` (in `wraithhabase.rfc`) is of type 2 among every
  shipped chunk tree's materials.

## What changes

- `exa_render::material_state`: type 2's mask is `0x400008`;
  `draws_smoke`.
- `exa_render::smoke`: the key words, the build rule, the alpha mirror;
  the unused smoke material and its scroll system are gone.
- `shadesmk.wesl` becomes a library; `exa.wesl` draws a `flags2::SMOKE`
  surface with it and discards under the distance fog.
- `exa_assets`: a type-2 record sets `flags2::SMOKE`.
- `exa-tool corpus materials`, `exa-tool shader --scene --smoke`.

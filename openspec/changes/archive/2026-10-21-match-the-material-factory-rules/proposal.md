# Proposal

## Why

The running game's material path (`ModelLoader::exa_material` over the
`shader_program` words) deviated from the original's material loader and
factory in five places: it never applied the loader's late opaque rewrite,
premultiplied the wrong records, doubled the emissive colour on the raw flags
instead of the type's plug-in-masked word, let the wrong records cast shadows,
and bound texture maps the factory leaves unbound. This is Phase 5 (visual
fidelity) of `docs/PLAN.md`, the RF_Materials and default-material-factory rows.

## What Changes

- The loader's late rewrite: a loaded destination nibble 3 becomes source
  ONE / destination ZERO (opaque) in refraction mode (loaded flags), or with
  opacity above 0.99, fall-off below 0.01 and no loaded opacity flag
  (`0x400000`, from the record or an opacity layer). It starts from the class
  after the alpha-test rewrite; the compares widen the stored f32 to f64.
- A record premultiplies its diffuse by the stored opacity when it is lit,
  its plug-in-masked word is not in refraction mode and its blend source (after
  the late rewrite) is ONE. An unlit record's `MatDif.rgb` is zero.
- The emissive colour doubles only when the masked word keeps `MatEmi`, the
  lit or specular bits and the emissive layer; an emissive-only record's
  alpha is its opacity, unclamped.
- Shadows: refraction mode always casts; otherwise opacity above 0.8 (f64)
  and a loaded destination nibble other than ONE; extra flag `0x4` clears both.
- Maps bind only where the factory binds them (emissive needs `MatEmi`,
  specular `0x100004`, normal `0x100807` in the material word, a diffuse map
  a lit record; a type-2 record keeps only the colour/opacity slot).

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: blend words, shadow casters and the factory's feature word.

## Impact

- `exa_assets::model` (`blended_class`, `casts_shadow`, `premultiplies`,
  `shaded_diffuse`, `emissive`, `factory_maps`); `exa_render::shader_program`
  (`feature_key`, `wants_normal_map`).
- `docs/RENDERING.md`, `docs/formats/FLB.md`, crate READMEs.
- Evidence: the original's loader and default material factory, checked
  against the assembly (f64 compares with NaN branches, plug-in masks); the
  tests are synthetic records only.
- Out of scope: the colour slot shared by the diffuse and opacity layers in
  the original (Exarust keeps two maps), the loader's texture-load result
  deciding the opacity flag, the rest of the factory (tables, `shadow_bias_scales`).

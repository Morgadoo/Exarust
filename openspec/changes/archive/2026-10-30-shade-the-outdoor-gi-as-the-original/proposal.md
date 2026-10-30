# Proposal

## Why

Outdoor arenas looked dim under the voxel GI: noon shadows came out dark
and a lightning strike lit the GI only faintly. Exarust shaded the GI's
light with the original's indoor curve everywhere, while the original's
`ambient` snippet switches to an outdoor curve under an open sky and
multiplies the result by the environment cube. Exarust also bound the
indoor cube (`genref01r`) to outdoor levels, whose faces are about 2.5×
darker than the open sky's `genref02r`.

Evidence (the shader library dumped from `shading.flb`, `ambient.glsl`,
`speculargi.glsl`, `shadehar_f.glsl`, `fog.glsl`; REA/Ghidra and the
objdump listing of the 0.9.5 build):

- Under `Env.Outdoor`, after the taps are averaged: a = |light|; with
  shadow taps the openness becomes max(1 − (1 − occ)·a·32, occ);
  b = (1 − a)²·0.08/(a + 0.035) + a; amb ×= Ambient.xyz·b·occ; `vgi` keeps
  that light; amb ×= EnvMap(nrm, lod 7)·(Ambient.w·5).
- Indoors b = (1 − a)²·0.03/(a + 0.016) + (0.47a + 0.53), no cube factor.
- Outdoors the reflections take `vgi` × Ambient.w × 2 (hair × 0.25), and
  the fog takes `vgi` for `amb`.
- The program switch `Outdoor` is bit 0x40 of the environment's program
  flags, set from the sky flag; `Ambient.w` is 8 indoors and 3 outdoors;
  the environment loads `genref02r` for an outdoor scene.
- The GI sky light's colour: the manager writes the light node's colour
  directly, nothing else writes that field, and the injection reads it
  for a directional light scaled by 3 and the 0.1 share, with no
  linearising step (the record fog colour is linear from the blend).

Phase: Phase 5 (world rendering): the voxel GI and the scene environment.

## What Changes

- `exa_render::shading` (`shading.wesl`): `gi_ambient_outdoor`, the outdoor
  openness, lift and cube factor, returning the ambient and the light
  before the cube; `gi_ambient` keeps the indoor curve in the original's
  order.
- `exa_render` world material (`exa.wesl`) and `exa_world` terrain
  (`terrain.wesl`): an `OUTDOOR` flag selects the outdoor shading; the
  reflections, hair reflection and fog take the light before the cube
  factor and `Ambient.w` 3 outdoors.
- `exa_render::gi`: the `GiOutdoor` resource (the sky flag and the level's
  cube); the GI systems set the flag on lit and terrain materials and bind
  the level's cube to every material with one.
- `exa_assets::model`: `sky_environment_map` (`genref02r`), the cube cache
  keyed by name.
- `exa_world`: the level load inserts `GiOutdoor` from its lighting.
- `exa_render::gi_ambient`: the curves in Rust with tests of the original's
  numbers, and checks that the shaders carry the same constants.

## Out of scope

- The tint of `genref02r` through `genref02rm` by the day record's sky
  tint (when and with which record the original applies it is not traced);
  the cube is bound as stored.
- `Ambient.w` 3 on the cube path outside the GI volume (it keeps 8).
- The `VXGISpec` cone trace and the local reflection layer.

## Impact

`exa_render` (shading.wesl, exa.wesl, gi.rs, gi_ambient.rs, material.rs),
`exa_world` (terrain, level load), `exa_assets` (model);
docs/RENDERING.md, crates/exa_render/README.md.

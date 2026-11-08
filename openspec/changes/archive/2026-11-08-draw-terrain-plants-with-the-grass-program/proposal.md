## Why

Gap lane `outdoor` (RF_Terrain, FLX_Terrain_Texrender2, TTerrainGrass). The
TTerrainGrass routine table found the grass hook's program builder missing:
Exarust drew its plants with the generic model program (legacy specular at
F0 0.04 × the specular colour, roughness 0.7, normals flipped on back faces),
and the hook's shading-quality switch of the normal-mapped variant was kept
only as a tested rule. The rating's "wind program" gap was a wrong premise.

## Evidence

REA (Exanima.exe). The grass hook's program builder compiles `shading_v`
with `shadegrs_f` (no other vertex program: no wind term exists). The hook's
static render state holds the material word `0x20017` (lit, `MatSpc`,
two-sided, `TexDif`) and MatDif (0.72, 0.68, 0.76), the diffuse rgb replaced
by the plant colour every draw; the quality switch adds `TexNrm` and
`TngtNrm` and binds `PlantSet_01N` only above shading quality 3 with the
texture loaded, every draw. `shadegrs_f`: roughness `clamp(1 − 3(g − 0.8r),
0.01, 0.99)²` of the diffuse sample, the fast light loop (`lightsfst`), no
back-face flip, ¾ of the specular sum added. The hook draws with depth
writes off (`glDepthMask(false)`, its load-table name checked).

## What Changes

- exa_render: `material::flags2::GRASS`; `exa.wesl` grass branch (normal,
  roughness, F0 0.04, ¾ specular); `shading.wesl` `light_fast`.
- exa_world: the plant material takes `GRASS`; `sync_plant_normal_map` sets
  or clears its normal-map bit by the shading quality.
- Docs: TERRAIN.md (wind premise corrected), RENDERING.md (Grass), crate
  READMEs.
- The composite terrain draw stays opt-in (`EXA_TERRAIN_SETS=1`): on
  arenao3 it matches the per-pixel draw and ran about 1 FPS lower of 21
  under a shared GPU, so it was not made the default.

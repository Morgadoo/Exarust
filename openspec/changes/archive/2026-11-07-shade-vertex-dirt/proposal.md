# Proposal

## Why

docs/PLAN.md Phase 5 (visual fidelity), surfaces milestone; the TMesh gap
lane (meshrender). The last TMesh rating's gap names the stored vertex
colours: the mesh build kept the colour stream on the CPU because Exarust
shaded no dirt mode but the wound blood. The original's GPU build copies the
stream into every coloured mesh's vertex buffer, and its PBR program shades
dirt modes 1 and 2 from the stream's alpha. `exa-tool corpus colours` finds
1 541 coloured meshes the running game shades with grime and 852 with wear,
among them arenao1's main level meshes, its tables and railings, and the
barrels, shelves, chairs, troughs and carts of exanima01.

## Evidence

REA (Ghidra listing and assembly of `Exanima.exe`):
- the material factory's dirt branch: with the diffuse feature bit, extra
  flags `0x7000` of `0x4000` bind the built-in speckle (blood), otherwise a
  record with the dirt feature bit and a loaded dirt texture binds it and
  sets the shader word's dirt field to 1, 2, 3 or 4 for `0`, `0x1000`,
  `0x2000`, `0x4000`;
- the material parser loads the dirt slot (`0x8000000`) in the same colour
  mode as the diffuse slot;
- the material uniform fill: for dirt modes other than blood, `MatDrt.w =
  2 − record 0x30` and `DrtUVs` = the diffuse texture's aspect, `(w/h, 1)`
  when `w/h ≥ 1`, else `(1, 1/(w/h))`;
- the GPU build's colour branch (stream type `0x100`) and the vertex
  program's `vrCol = vCol / 255`;
- the captured shader library: `shadeopt_f`'s `Shd.DirtMode = 1` and `= 2`
  branches inside `Obj.VrtCol` (roughness, F0 and diffuse colour), and no
  mode 1–3 branch in `shading_f`, `shadehar_f`, `shadesss_f`, `shadetrl_f`.

## What Changes

- `exa_render::material::DirtMode` (grime, wear): the record's mode under
  the PBR program, the take and aspect, the per-pixel rule as a tested
  model; `ExaMaterial::with_dirt`, `flags2::DIRT_GRIME`, `DIRT_WORN`,
  `GREY_DIRT`, `ExaParams::dirt`.
- `exa.wesl`: the two branches, reading the vertex alpha, the dirt map and
  the bound diffuse map's aspect.
- `exa_assets::model`: the material binds the dirt map for modes 1 and 2;
  the mesh build carries the stored colour stream as four bytes a vertex
  (`COLOUR_STREAM`) and assembly keeps it only for a dirt surface.
- `exa-tool corpus colours`: grime and wear columns and examples.

## Out of scope

- Mode 3 (no program branch: no change on screen), and the hair, skin,
  translucent and legacy programs (no mode 1–3 branch).
- Skinned linked-body members (built without colours).
- Wetness (Exarust has no wet surfaces yet).

## Impact

`exa_render`, `exa_assets`, `exa_tool`; docs/RENDERING.md (Dirt); the
crate READMEs. Simulation untouched (exanima01 and arena duel traces
identical to main).
